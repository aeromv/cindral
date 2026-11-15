// SPDX-License-Identifier: MIT OR Apache-2.0

//! Face merging: coplanar neighbouring quads that would shade identically are
//! joined into one larger quad (a greedy merge), so the GPU processes fewer
//! vertices. This is the main cost of the terrain on a software rasterizer or
//! a weak integrated GPU, where vertex work is serial (`docs/perf-log.md`,
//! 2026-11-14).
//!
//! Two quads are merged only when every vertex carries the same normal,
//! texture layer, light payload (block, sky, AO, shade) and tint, so the
//! interpolated light, tint and fog of the merged quad equal those of the
//! quads it replaces. The texture repeats over the merged quad: its
//! coordinates run past `1.0` and the terrain shader wraps them with `fract`
//! (`sample_block` in `terrain.wgsl`), keeping the derivatives of the
//! unwrapped coordinates so the mip level does not change at the seams.
//!
//! Quads that are not axis-aligned rectangles, whose texture coordinates are
//! not an affine function of the position, that run outside `0..=1`, or that
//! share their exact rectangle with another quad (an overlay drawn over a base
//! face, such as the grass side) are kept as they are. Only the opaque pass is
//! merged: the blended pass is sorted and its liquid corners vary.
//!
//! The order of the quads is kept (a merged quad takes the place of its first
//! quad) and the section spans are rebuilt, so cave culling and the
//! solid/cutout split of the renderer work unchanged.

use std::collections::{HashMap, HashSet};

use crate::mesh::{Mesh, VertexSpan};
use crate::Vertex;

/// Slack on texture coordinates that are meant to be `0.0` or `1.0`.
const UV_EPSILON: f32 = 1.0 / 1024.0;

/// Merges the opaque quads of `mesh` section by section; returns how many
/// quads were removed.
pub fn merge_faces(mesh: &mut Mesh) -> usize {
    let quads_before = mesh.vertices.len() / 4;
    if quads_before < 2 {
        return 0;
    }
    let spans: Vec<VertexSpan> = if mesh.sections.is_empty() {
        vec![VertexSpan {
            start: 0,
            len: mesh.vertices.len() as u32,
        }]
    } else {
        mesh.sections.iter().map(|section| section.opaque).collect()
    };
    let mut out: Vec<Vertex> = Vec::with_capacity(mesh.vertices.len());
    let mut new_spans = Vec::with_capacity(spans.len());
    for span in &spans {
        let start = span.start as usize;
        let end = start + span.len as usize;
        let first = out.len();
        merge_span(&mesh.vertices[start..end], &mut out);
        new_spans.push(VertexSpan {
            start: first as u32,
            len: (out.len() - first) as u32,
        });
    }
    if !mesh.sections.is_empty() {
        for (section, span) in mesh.sections.iter_mut().zip(new_spans) {
            section.opaque = span;
        }
    }
    let quads_after = out.len() / 4;
    mesh.vertices = out;
    mesh.indices.clear();
    mesh.indices.reserve(quads_after * 6);
    for quad in 0..quads_after as u32 {
        let base = quad * 4;
        mesh.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    quads_before - quads_after
}

/// What two quads must share to be merged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Key {
    axis: u8,
    plane: u32,
    normal: [u32; 3],
    layer: u32,
    light: u32,
    tint: u32,
    /// The texture-coordinate slopes along the two tangent axes.
    slope: [u32; 4],
    /// The fractional texture offset, in 1/4096: quads whose offsets differ by
    /// whole textures tile into each other.
    phase: [i32; 2],
    /// Which corner each of the four vertices is (keeps the winding).
    corners: [u8; 4],
}

/// One rectangle of a merge group, in the two tangent axes.
#[derive(Debug, Clone, Copy)]
struct Rect {
    /// `t1` min, `t1` max, `t2` min, `t2` max.
    bounds: [f32; 4],
    /// The texture coordinates at the `(t1 min, t2 min)` corner.
    anchor_uv: [f64; 2],
    /// The index of the first quad it holds, for the output order.
    order: usize,
    /// The quad its vertex attributes come from.
    template: usize,
}

/// A quad that can merge: its key and rectangle.
struct Candidate {
    key: Key,
    rect: Rect,
    /// `[du/dt1, dv/dt1, du/dt2, dv/dt2]`.
    slope: [f64; 4],
}

/// Merges one section's opaque quads into `out`.
fn merge_span(vertices: &[Vertex], out: &mut Vec<Vertex>) {
    let quads: Vec<&[Vertex]> = vertices.chunks_exact(4).collect();
    let candidates: Vec<Option<Candidate>> = quads
        .iter()
        .enumerate()
        .map(|(index, quad)| analyse(quad, index))
        .collect();

    // An exact rectangle used twice on the same side of a plane is an overlay
    // over a base face: merging either could let them z-fight.
    let mut seen = HashSet::new();
    let mut doubled = HashSet::new();
    for candidate in candidates.iter().flatten() {
        let place = place_key(candidate);
        if !seen.insert(place) {
            doubled.insert(place);
        }
    }

    let mut groups: HashMap<Key, (Vec<Rect>, [f64; 4])> = HashMap::new();
    // (order, merged vertices) of everything emitted.
    let mut emitted: Vec<(usize, [Vertex; 4])> = Vec::with_capacity(quads.len());
    for (index, candidate) in candidates.into_iter().enumerate() {
        match candidate {
            Some(candidate) if !doubled.contains(&place_key(&candidate)) => {
                groups
                    .entry(candidate.key)
                    .or_insert_with(|| (Vec::new(), candidate.slope))
                    .0
                    .push(candidate.rect);
            }
            _ => {
                let quad = quads[index];
                emitted.push((index, [quad[0], quad[1], quad[2], quad[3]]));
            }
        }
    }

    for (key, (rects, slope)) in groups {
        for rect in merge_rects(rects) {
            emitted.push((
                rect.order,
                build_quad(&key, &rect, slope, quads[rect.template]),
            ));
        }
    }
    emitted.sort_unstable_by_key(|(order, _)| *order);
    for (_, quad) in emitted {
        out.extend_from_slice(&quad);
    }
}

/// Where a candidate lies: plane, side and exact rectangle.
fn place_key(candidate: &Candidate) -> (u8, u32, [u32; 3], [u32; 4]) {
    (
        candidate.key.axis,
        candidate.key.plane,
        candidate.key.normal,
        candidate.rect.bounds.map(f32::to_bits),
    )
}

/// The merge key and rectangle of a quad, or `None` when it cannot merge.
fn analyse(quad: &[Vertex], index: usize) -> Option<Candidate> {
    let first = quad[0];
    if quad.iter().any(|vertex| {
        vertex.normal != first.normal
            || vertex.layer != first.layer
            || vertex.light != first.light
            || vertex.tint != first.tint
    }) {
        return None;
    }
    let axis = unit_axis(first.normal)?;
    let plane = first.position[axis];
    if quad.iter().any(|vertex| vertex.position[axis] != plane) {
        return None;
    }
    if quad.iter().any(|vertex| {
        vertex
            .uv
            .iter()
            .any(|&c| !(-UV_EPSILON..=1.0 + UV_EPSILON).contains(&c))
    }) {
        return None;
    }
    let (t1, t2) = ((axis + 1) % 3, (axis + 2) % 3);
    let lo1 = quad
        .iter()
        .map(|v| v.position[t1])
        .fold(f32::INFINITY, f32::min);
    let hi1 = quad
        .iter()
        .map(|v| v.position[t1])
        .fold(f32::NEG_INFINITY, f32::max);
    let lo2 = quad
        .iter()
        .map(|v| v.position[t2])
        .fold(f32::INFINITY, f32::min);
    let hi2 = quad
        .iter()
        .map(|v| v.position[t2])
        .fold(f32::NEG_INFINITY, f32::max);
    if lo1 >= hi1 || lo2 >= hi2 {
        return None;
    }
    // Each vertex must sit on one of the four corners, each corner used once.
    let mut corners = [0u8; 4];
    let mut used = [false; 4];
    let mut at = [0usize; 4];
    for (slot, vertex) in quad.iter().enumerate() {
        let a = corner_side(vertex.position[t1], lo1, hi1)?;
        let b = corner_side(vertex.position[t2], lo2, hi2)?;
        let corner = a | (b << 1);
        if used[corner as usize] {
            return None;
        }
        used[corner as usize] = true;
        corners[slot] = corner;
        at[corner as usize] = slot;
    }
    let uv = |corner: usize| quad[at[corner]].uv.map(f64::from);
    let (c00, c10, c01, c11) = (uv(0), uv(1), uv(2), uv(3));
    // Affine over the rectangle: the diagonals agree.
    for k in 0..2 {
        if ((c00[k] + c11[k]) - (c10[k] + c01[k])).abs() > 1e-5 {
            return None;
        }
    }
    let (w, h) = (f64::from(hi1 - lo1), f64::from(hi2 - lo2));
    let slope = [
        (c10[0] - c00[0]) / w,
        (c10[1] - c00[1]) / w,
        (c01[0] - c00[0]) / h,
        (c01[1] - c00[1]) / h,
    ];
    // The texture offset at the world origin of the tangent axes, kept only
    // modulo one texture.
    let phase = [0, 1].map(|k| {
        let offset = c00[k] - slope[k] * f64::from(lo1) - slope[k + 2] * f64::from(lo2);
        let steps = (offset * 4096.0).round() as i64;
        steps.rem_euclid(4096) as i32
    });
    let key = Key {
        axis: axis as u8,
        plane: plane.to_bits(),
        normal: first.normal.map(f32::to_bits),
        layer: first.layer,
        light: first.light,
        tint: first.tint,
        slope: slope.map(|s| (s as f32).to_bits()),
        phase,
        corners,
    };
    Some(Candidate {
        key,
        rect: Rect {
            bounds: [lo1, hi1, lo2, hi2],
            anchor_uv: c00,
            order: index,
            template: index,
        },
        slope,
    })
}

/// `0` for the low side, `1` for the high side, `None` for neither.
fn corner_side(value: f32, lo: f32, hi: f32) -> Option<u8> {
    if value == lo {
        Some(0)
    } else if value == hi {
        Some(1)
    } else {
        None
    }
}

/// The axis of a unit axis-aligned normal.
fn unit_axis(normal: [f32; 3]) -> Option<usize> {
    let mut axis = None;
    for (index, component) in normal.iter().enumerate() {
        if *component == 0.0 {
            continue;
        }
        if component.abs() != 1.0 || axis.is_some() {
            return None;
        }
        axis = Some(index);
    }
    axis
}

/// Greedy merge of one group: rows along `t1` first, then the rows along `t2`.
fn merge_rects(mut rects: Vec<Rect>) -> Vec<Rect> {
    let by_t2_rows = |a: &Rect, b: &Rect| {
        a.bounds[2]
            .total_cmp(&b.bounds[2])
            .then(a.bounds[3].total_cmp(&b.bounds[3]))
            .then(a.bounds[0].total_cmp(&b.bounds[0]))
    };
    rects.sort_by(by_t2_rows);
    let rects = sweep(
        rects,
        |a, b| {
            a.bounds[2] == b.bounds[2] && a.bounds[3] == b.bounds[3] && a.bounds[1] == b.bounds[0]
        },
        1,
    );
    let mut rects = rects;
    rects.sort_by(|a, b| {
        a.bounds[0]
            .total_cmp(&b.bounds[0])
            .then(a.bounds[1].total_cmp(&b.bounds[1]))
            .then(a.bounds[2].total_cmp(&b.bounds[2]))
    });
    sweep(
        rects,
        |a, b| {
            a.bounds[0] == b.bounds[0] && a.bounds[1] == b.bounds[1] && a.bounds[3] == b.bounds[2]
        },
        3,
    )
}

/// Joins each rectangle into the previous one while `joins` holds, growing the
/// bound at index `grow`.
fn sweep(rects: Vec<Rect>, joins: impl Fn(&Rect, &Rect) -> bool, grow: usize) -> Vec<Rect> {
    let mut out: Vec<Rect> = Vec::with_capacity(rects.len());
    for rect in rects {
        match out.last_mut() {
            Some(last) if joins(last, &rect) => {
                last.bounds[grow] = rect.bounds[grow];
                if rect.order < last.order {
                    last.order = rect.order;
                }
            }
            _ => out.push(rect),
        }
    }
    out
}

/// The four vertices of a merged rectangle, in the corner order of the key.
fn build_quad(key: &Key, rect: &Rect, slope: [f64; 4], template: &[Vertex]) -> [Vertex; 4] {
    let axis = key.axis as usize;
    let (t1, t2) = ((axis + 1) % 3, (axis + 2) % 3);
    let [lo1, hi1, lo2, hi2] = rect.bounds;
    let mut quad = [template[0]; 4];
    for (slot, vertex) in quad.iter_mut().enumerate() {
        let corner = key.corners[slot];
        let p1 = if corner & 1 == 0 { lo1 } else { hi1 };
        let p2 = if corner & 2 == 0 { lo2 } else { hi2 };
        vertex.position[t1] = p1;
        vertex.position[t2] = p2;
        // Relative to the anchor so the texture stays exact far from the origin.
        let d1 = f64::from(p1) - f64::from(lo1);
        let d2 = f64::from(p2) - f64::from(lo2);
        vertex.uv = [
            (rect.anchor_uv[0] + slope[0] * d1 + slope[2] * d2) as f32,
            (rect.anchor_uv[1] + slope[1] * d1 + slope[3] * d2) as f32,
        ];
    }
    quad
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{tint, VertexLight};

    /// An upward face of the unit block at `(x, y, z)` with the usual corner
    /// order and `u = x`, `v = z` inside the block.
    fn top(x: f32, y: f32, z: f32, layer: u32, light: u32) -> [Vertex; 4] {
        let v = |dx: f32, dz: f32| Vertex {
            position: [x + dx, y + 1.0, z + dz],
            normal: [0.0, 1.0, 0.0],
            uv: [dx, dz],
            layer,
            light,
            tint: tint::WHITE,
        };
        [v(0.0, 0.0), v(0.0, 1.0), v(1.0, 1.0), v(1.0, 0.0)]
    }

    fn mesh_of(quads: &[[Vertex; 4]]) -> Mesh {
        let mut mesh = Mesh::default();
        for quad in quads {
            mesh.push_quad(false, *quad);
        }
        mesh
    }

    /// The texel a point of the plane would sample, wrapped like the shader.
    fn sample(mesh: &Mesh, x: f32, z: f32) -> Option<(u32, [f32; 2])> {
        for quad in mesh.vertices.chunks_exact(4) {
            let (lx, hx) = (
                quad.iter()
                    .map(|v| v.position[0])
                    .fold(f32::INFINITY, f32::min),
                quad.iter()
                    .map(|v| v.position[0])
                    .fold(f32::NEG_INFINITY, f32::max),
            );
            let (lz, hz) = (
                quad.iter()
                    .map(|v| v.position[2])
                    .fold(f32::INFINITY, f32::min),
                quad.iter()
                    .map(|v| v.position[2])
                    .fold(f32::NEG_INFINITY, f32::max),
            );
            if (lx..hx).contains(&x) && (lz..hz).contains(&z) {
                let c = quad
                    .iter()
                    .find(|v| v.position[0] == lx && v.position[2] == lz)?;
                let u = c.uv[0] + (x - lx);
                let w = c.uv[1] + (z - lz);
                return Some((quad[0].layer, [u.fract(), w.fract()]));
            }
        }
        None
    }

    #[test]
    fn a_flat_floor_becomes_one_quad_and_samples_the_same_texels() {
        let light = VertexLight::DAY.pack();
        let quads: Vec<_> = (0..16)
            .flat_map(|x| (0..16).map(move |z| top(x as f32, 63.0, z as f32, 3, light)))
            .collect();
        let before = mesh_of(&quads);
        let mut after = before.clone();
        assert_eq!(merge_faces(&mut after), 255);
        assert_eq!(after.vertices.len(), 4);
        assert_eq!(after.indices, vec![0, 1, 2, 0, 2, 3]);
        for (x, z) in [(0.25, 0.25), (5.5, 9.75), (15.9, 15.9), (7.0625, 0.5)] {
            assert_eq!(sample(&before, x, z), sample(&after, x, z), "at {x}, {z}");
        }
    }

    #[test]
    fn different_light_layer_or_overlays_are_not_merged() {
        let day = VertexLight::DAY.pack();
        let dark = VertexLight::new(0, 4, 15, true).pack();
        let mut mesh = mesh_of(&[
            top(0.0, 0.0, 0.0, 1, day),
            top(1.0, 0.0, 0.0, 1, dark),
            top(2.0, 0.0, 0.0, 2, day),
        ]);
        assert_eq!(merge_faces(&mut mesh), 0);

        // A base and an overlay on the same rectangles stay four quads.
        let mut overlay = mesh_of(&[
            top(0.0, 0.0, 0.0, 1, day),
            top(0.0, 0.0, 0.0, 5, day),
            top(1.0, 0.0, 0.0, 1, day),
            top(1.0, 0.0, 0.0, 5, day),
        ]);
        assert_eq!(merge_faces(&mut overlay), 0);
    }

    #[test]
    fn the_quad_order_and_winding_are_kept() {
        let day = VertexLight::DAY.pack();
        let mut mesh = mesh_of(&[
            top(0.0, 0.0, 0.0, 1, day),
            top(5.0, 0.0, 5.0, 9, day),
            top(1.0, 0.0, 0.0, 1, day),
        ]);
        assert_eq!(merge_faces(&mut mesh), 1);
        assert_eq!(mesh.vertices[0].layer, 1);
        assert_eq!(mesh.vertices[4].layer, 9);
        // Same corner order as the source quad: (lo, lo), (lo, hi), (hi, hi), (hi, lo).
        let corners: Vec<_> = mesh.vertices[..4]
            .iter()
            .map(|v| (v.position[0], v.position[2]))
            .collect();
        assert_eq!(
            corners,
            vec![(0.0, 0.0), (0.0, 1.0), (2.0, 1.0), (2.0, 0.0)]
        );
        assert_eq!(mesh.vertices[2].uv, [2.0, 1.0]);
    }

    #[test]
    fn far_from_the_origin_the_texture_stays_exact() {
        let day = VertexLight::DAY.pack();
        let base = 1_000_000.0;
        let mut mesh = mesh_of(&[
            top(base, 0.0, base, 1, day),
            top(base + 1.0, 0.0, base, 1, day),
        ]);
        assert_eq!(merge_faces(&mut mesh), 1);
        let uvs: Vec<_> = mesh.vertices.iter().map(|v| v.uv).collect();
        assert_eq!(uvs, vec![[0.0, 0.0], [0.0, 1.0], [2.0, 1.0], [2.0, 0.0]]);
    }

    #[test]
    fn section_spans_are_rebuilt() {
        let day = VertexLight::DAY.pack();
        let mut mesh = Mesh::default();
        let start = mesh.section_start();
        mesh.push_quad(false, top(0.0, 0.0, 0.0, 1, day));
        mesh.push_quad(false, top(1.0, 0.0, 0.0, 1, day));
        mesh.finish_section(start, crate::visibility::Connectivity::OPEN);
        let start = mesh.section_start();
        mesh.push_quad(false, top(0.0, 16.0, 0.0, 1, day));
        mesh.finish_section(start, crate::visibility::Connectivity::OPEN);
        assert_eq!(merge_faces(&mut mesh), 1);
        assert_eq!(mesh.sections[0].opaque, VertexSpan { start: 0, len: 4 });
        assert_eq!(mesh.sections[1].opaque, VertexSpan { start: 4, len: 4 });
    }
}
