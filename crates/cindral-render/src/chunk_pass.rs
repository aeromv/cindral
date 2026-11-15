// SPDX-License-Identifier: MIT OR Apache-2.0

//! The GPU side of a chunk mesh pass: vertices only, solid quads first, drawn
//! with one shared quad index buffer (F2-2a, F2-2d).
//!
//! Every quad the mesher emits has the same index pattern
//! `[0, 1, 2, 0, 2, 3] + 4 * quad`, so one static index buffer serves every
//! chunk and a mesh uploads vertices only. The opaque pass is also split in two
//! ranges: quads whose texture has no texel the cutout test could discard
//! (SOLID) come first and are drawn by a pipeline without `discard`, which
//! keeps early depth rejection on; the rest (CUTOUT) follow.

use std::ops::Range;

use cindral_mesh::{Vertex, VertexSpan};
use wgpu::util::DeviceExt;

/// Indices per quad.
const INDICES_PER_QUAD: u32 = 6;
/// Vertices per quad.
const VERTICES_PER_QUAD: usize = 4;
/// The alpha below which the terrain shader discards a texel (0.5 in 0..=255).
const CUTOUT_ALPHA: u8 = 128;
/// Quads the shared index buffer starts with; it doubles when a mesh needs more.
const MIN_QUADS: u32 = 4096;

/// Sections per column.
pub(crate) const SECTIONS: usize = 16;

/// A run of quads of a pass.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct QuadRange {
    pub(crate) start: u32,
    pub(crate) len: u32,
}

/// The facing buckets of a section's quads, in their order in the buffer:
/// +X, -X, +Y, -Y, +Z, -Z, then the quads whose normal is not an axis
/// (crossed plants, rotated elements), which are always drawn.
pub(crate) const FACINGS: usize = 7;
/// The bucket of the quads that are always drawn.
const UNALIGNED: usize = 6;
/// Every bucket visible.
pub(crate) const ALL_FACINGS: u8 = 0x7F;

/// The bucket of a quad, from the normal of its first vertex.
fn facing(normal: [f32; 3]) -> usize {
    match normal {
        [1.0, 0.0, 0.0] => 0,
        [-1.0, 0.0, 0.0] => 1,
        [0.0, 1.0, 0.0] => 2,
        [0.0, -1.0, 0.0] => 3,
        [0.0, 0.0, 1.0] => 4,
        [0.0, 0.0, -1.0] => 5,
        _ => UNALIGNED,
    }
}

/// The facing buckets that can face an eye at `eye` (relative to the corner of
/// the 16-block section box, the box spanning `0..16` on each axis), as bits.
///
/// A quad with normal +X whose plane is at `x` faces the eye only when
/// `eye.x > x`; every plane of a +X quad of the section is at `x >= 0`, so with
/// `eye.x <= 0` none of them does and the bucket is skipped. The back faces
/// would be culled by the pipeline anyway, after their vertices were shaded:
/// skipping them first saves that work (Sodium's facing culling).
#[must_use]
pub(crate) fn visible_facings(eye: [f64; 3]) -> u8 {
    let mut bits = 1 << UNALIGNED;
    for (axis, &at) in eye.iter().enumerate() {
        if at > 0.0 {
            bits |= 1 << (2 * axis);
        }
        if at < 16.0 {
            bits |= 1 << (2 * axis + 1);
        }
    }
    bits
}

/// Where one section's quads are in an uploaded pass.
///
/// A sorted opaque pass (the mesher's spans and the cutout flags were known)
/// is laid out group by group (solid, then cutout), then by facing bucket,
/// then by section: the +X quads of every section of the column are
/// contiguous, so a bucket that the eye sees in every visible section is one
/// draw. Its ranges are in `solid_facings` / `cutout_facings`. An unsorted
/// pass keeps one range per section and group in `solid` / `cutout`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct SectionQuads {
    pub(crate) solid: QuadRange,
    pub(crate) cutout: QuadRange,
    /// Whether the facing ranges are used instead of `solid` and `cutout`.
    pub(crate) sorted: bool,
    /// The solid quads of each facing bucket.
    pub(crate) solid_facings: [QuadRange; FACINGS],
    /// The cutout quads of each facing bucket.
    pub(crate) cutout_facings: [QuadRange; FACINGS],
}

/// Which quads of a section to draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Group {
    /// The quads with no alpha test. A pass that is not split (the blended
    /// one) keeps all its quads here.
    Solid,
    /// The quads that discard texels.
    Cutout,
}

/// One uploaded pass of a chunk mesh.
pub(crate) struct ChunkPass {
    pub(crate) vertex_buffer: wgpu::Buffer,
    /// Quads at the start of the buffer that need no alpha test.
    pub(crate) solid_quads: u32,
    /// All quads of the buffer.
    pub(crate) quads: u32,
    /// The cutout flags this pass was sorted with: [`Renderer`](crate::Renderer)
    /// bumps its epoch when the block array changes, which makes the sort
    /// stale until the mesh is uploaded again.
    pub(crate) epoch: u32,
    /// The quads of each section, when the mesh came with its spans; `None`
    /// draws the pass whole.
    pub(crate) sections: Option<Box<[SectionQuads; SECTIONS]>>,
}

impl ChunkPass {
    /// Calls `draw` with the index range of every run of the sections in
    /// `mask` (bit `s` = section `s`) for `group`; sections that touch in the
    /// buffer are merged into one run. A pass without spans is drawn whole
    /// whenever the mask is not empty.
    pub(crate) fn visible_runs(
        &self,
        (mask, facings): (u16, &dyn Fn(usize) -> u8),
        group: Group,
        mut draw: impl FnMut(Range<u32>),
    ) {
        let Some(sections) = &self.sections else {
            if mask != 0 {
                let range = match group {
                    Group::Solid => self.solid_indices(),
                    Group::Cutout => self.cutout_indices(),
                };
                if !range.is_empty() {
                    draw(range);
                }
            }
            return;
        };
        collect_runs(sections, (mask, facings), group, &mut draw);
    }

    /// The indices of the SOLID quads.
    pub(crate) fn solid_indices(&self) -> Range<u32> {
        0..self.solid_quads * INDICES_PER_QUAD
    }

    /// The indices of the CUTOUT quads.
    pub(crate) fn cutout_indices(&self) -> Range<u32> {
        self.solid_quads * INDICES_PER_QUAD..self.quads * INDICES_PER_QUAD
    }
}

/// The walk of [`ChunkPass::visible_runs`] over the sections of a pass.
fn collect_runs(
    sections: &[SectionQuads; SECTIONS],
    (mask, facings): (u16, &dyn Fn(usize) -> u8),
    group: Group,
    draw: &mut impl FnMut(Range<u32>),
) {
    let mut run: Option<QuadRange> = None;
    let mut add = |quads: QuadRange, visible: bool, run: &mut Option<QuadRange>| {
        if quads.len == 0 {
            // Nothing to draw: it neither joins nor breaks a run.
            return;
        }
        *run = if visible {
            match *run {
                Some(open) if open.start + open.len == quads.start => Some(QuadRange {
                    start: open.start,
                    len: open.len + quads.len,
                }),
                open => {
                    flush(open, draw);
                    Some(quads)
                }
            }
        } else {
            flush(*run, draw);
            None
        };
    };
    let shown = |index: usize| mask >> index & 1 == 1;
    if sections.iter().any(|section| section.sorted) {
        let visible: [u8; SECTIONS] =
            std::array::from_fn(|index| if shown(index) { facings(index) } else { 0 });
        for bucket in 0..FACINGS {
            for (index, section) in sections.iter().enumerate() {
                let quads = match group {
                    Group::Solid => section.solid_facings[bucket],
                    Group::Cutout => section.cutout_facings[bucket],
                };
                add(quads, visible[index] >> bucket & 1 == 1, &mut run);
            }
        }
    } else {
        for (index, section) in sections.iter().enumerate() {
            let quads = match group {
                Group::Solid => section.solid,
                Group::Cutout => section.cutout,
            };
            add(quads, shown(index), &mut run);
        }
    }
    flush(run, draw);
}

/// Draws the run, if any, as indices.
fn flush(run: Option<QuadRange>, draw: &mut impl FnMut(Range<u32>)) {
    if let Some(run) = run {
        draw(run.start * INDICES_PER_QUAD..(run.start + run.len) * INDICES_PER_QUAD);
    }
}

/// Uploads one pass, or returns `None` when it has no whole quad (wgpu rejects
/// a zero-sized buffer).
///
/// With `cutout` (one flag per texture layer, and the epoch of the flags) the quads are reordered, solid
/// first, through `scratch`, which keeps its capacity across calls. Without
/// it all quads count as solid (the blended pass, which is not split).
pub(crate) fn create_chunk_pass(
    device: &wgpu::Device,
    label: &str,
    vertices: &[Vertex],
    spans: Option<&[VertexSpan]>,
    cutout: Option<(&[bool], u32)>,
    scratch: &mut Vec<Vertex>,
) -> Option<ChunkPass> {
    let quads = (vertices.len() / VERTICES_PER_QUAD) as u32;
    if quads == 0 {
        return None;
    }
    let vertices = &vertices[..quads as usize * VERTICES_PER_QUAD];
    let spans = spans.filter(|spans| spans_tile(spans, vertices.len()));
    let (contents, solid_quads, epoch, sections) = match (cutout, spans) {
        (Some((flags, epoch)), Some(spans)) => {
            let (solid, sections) = split_sections(vertices, spans, flags, scratch);
            (scratch.as_slice(), solid, epoch, Some(sections))
        }
        (Some((flags, epoch)), None) => {
            let solid = split_solid_cutout(vertices, flags, scratch);
            (scratch.as_slice(), solid, epoch, None)
        }
        (None, Some(spans)) => (vertices, quads, 0, Some(unsplit_sections(spans))),
        (None, None) => (vertices, quads, 0, None),
    };
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: bytemuck::cast_slice(contents),
        usage: wgpu::BufferUsages::VERTEX,
    });
    Some(ChunkPass {
        vertex_buffer,
        solid_quads,
        quads,
        epoch,
        sections,
    })
}

/// Whether `spans` are one per section and tile `vertices` end to end in
/// whole quads (what the mesher produces); anything else is ignored.
fn spans_tile(spans: &[VertexSpan], vertices: usize) -> bool {
    let mut next = 0_usize;
    let tiles = spans.len() == SECTIONS
        && spans.iter().all(|span| {
            let aligned = (span.len as usize).is_multiple_of(VERTICES_PER_QUAD);
            let contiguous = span.start as usize == next;
            next += span.len as usize;
            aligned && contiguous
        });
    tiles && next == vertices
}

/// The sections of a pass that is not split: every quad is "solid".
fn unsplit_sections(spans: &[VertexSpan]) -> Box<[SectionQuads; SECTIONS]> {
    let mut sections = Box::new([SectionQuads::default(); SECTIONS]);
    for (section, span) in sections.iter_mut().zip(spans) {
        section.solid = QuadRange {
            start: span.start / VERTICES_PER_QUAD as u32,
            len: span.len / VERTICES_PER_QUAD as u32,
        };
    }
    sections
}

/// Like [`split_solid_cutout`], for a pass with section spans, laid out as
/// [`SectionQuads`] describes: solid then cutout, each by facing bucket, each
/// bucket section by section; returns the number of solid quads and where
/// each section's quads went. Quads keep their relative order inside a bucket
/// of a section, so overlays still follow the faces they cover.
pub(crate) fn split_sections(
    vertices: &[Vertex],
    spans: &[VertexSpan],
    cutout: &[bool],
    out: &mut Vec<Vertex>,
) -> (u32, Box<[SectionQuads; SECTIONS]>) {
    let is_cutout = |quad: &[Vertex]| cutout.get(quad[0].layer as usize).copied().unwrap_or(true);
    let quads_of = |span: &VertexSpan| {
        vertices[span.start as usize..(span.start + span.len) as usize]
            .chunks_exact(VERTICES_PER_QUAD)
    };
    let mut sections = Box::new([SectionQuads::default(); SECTIONS]);
    for section in sections.iter_mut() {
        section.sorted = true;
    }
    out.clear();
    out.reserve(vertices.len());
    let mut solid_total = 0;
    for wants_cutout in [false, true] {
        for bucket in 0..FACINGS {
            for (section, span) in sections.iter_mut().zip(spans) {
                let start = (out.len() / VERTICES_PER_QUAD) as u32;
                for quad in quads_of(span).filter(|quad| {
                    is_cutout(quad) == wants_cutout && facing(quad[0].normal) == bucket
                }) {
                    out.extend_from_slice(quad);
                }
                let range = QuadRange {
                    start,
                    len: (out.len() / VERTICES_PER_QUAD) as u32 - start,
                };
                if wants_cutout {
                    section.cutout_facings[bucket] = range;
                } else {
                    section.solid_facings[bucket] = range;
                }
            }
        }
        if !wants_cutout {
            solid_total = (out.len() / VERTICES_PER_QUAD) as u32;
        }
    }
    (solid_total, sections)
}

/// Copies the quads of `vertices` into `out`, solid ones first (in their
/// order), and returns how many are solid.
///
/// A quad is cutout when the layer of its first vertex is flagged, or is
/// unknown: the safe side is the pipeline that discards.
pub(crate) fn split_solid_cutout(
    vertices: &[Vertex],
    cutout: &[bool],
    out: &mut Vec<Vertex>,
) -> u32 {
    let is_cutout = |quad: &[Vertex]| cutout.get(quad[0].layer as usize).copied().unwrap_or(true);
    out.clear();
    out.reserve(vertices.len());
    for quad in vertices.chunks_exact(VERTICES_PER_QUAD) {
        if !is_cutout(quad) {
            out.extend_from_slice(quad);
        }
    }
    let solid = (out.len() / VERTICES_PER_QUAD) as u32;
    for quad in vertices.chunks_exact(VERTICES_PER_QUAD) {
        if is_cutout(quad) {
            out.extend_from_slice(quad);
        }
    }
    solid
}

/// Which layers of a texture array have a texel the cutout test discards.
///
/// `pixels` is layer-major RGBA8, `layer_bytes` the size of one layer.
pub(crate) fn cutout_layers(pixels: &[u8], layer_bytes: usize) -> Vec<bool> {
    if layer_bytes == 0 {
        return Vec::new();
    }
    pixels
        .chunks_exact(layer_bytes)
        .map(|layer| layer.chunks_exact(4).any(|texel| texel[3] < CUTOUT_ALPHA))
        .collect()
}

/// The index pattern of `quads` quads.
fn quad_index_pattern(quads: u32) -> Vec<u32> {
    (0..quads)
        .flat_map(|quad| {
            let base = quad * VERTICES_PER_QUAD as u32;
            [base, base + 1, base + 2, base, base + 2, base + 3]
        })
        .collect()
}

/// The index buffer shared by every chunk pass.
pub(crate) struct QuadIndices {
    buffer: wgpu::Buffer,
    quads: u32,
}

impl QuadIndices {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        Self {
            buffer: Self::create(device, MIN_QUADS),
            quads: MIN_QUADS,
        }
    }

    fn create(device: &wgpu::Device, quads: u32) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("shared quad indices"),
            contents: bytemuck::cast_slice(&quad_index_pattern(quads)),
            usage: wgpu::BufferUsages::INDEX,
        })
    }

    /// Makes sure the buffer covers a mesh of `quads` quads, doubling it as
    /// needed.
    pub(crate) fn ensure(&mut self, device: &wgpu::Device, quads: u32) {
        if quads > self.quads {
            self.quads = quads.next_power_of_two();
            self.buffer = Self::create(device, self.quads);
        }
    }

    /// Binds the buffer for the draws that follow.
    pub(crate) fn bind(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_index_buffer(self.buffer.slice(..), wgpu::IndexFormat::Uint32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quad(layer: u32, x: f32) -> [Vertex; 4] {
        let vertex = |corner: f32| Vertex {
            position: [x, corner, 0.0],
            normal: [0.0, 1.0, 0.0],
            uv: [0.0, 0.0],
            layer,
            light: 0,
            tint: 0,
        };
        [vertex(0.0), vertex(1.0), vertex(2.0), vertex(3.0)]
    }

    #[test]
    fn solid_quads_come_first_and_keep_their_order() {
        let flags = [false, true, false];
        // Layers: solid, cutout, solid, cutout, solid.
        let input: Vec<Vertex> = [(0, 0.0), (1, 1.0), (2, 2.0), (1, 3.0), (0, 4.0)]
            .iter()
            .flat_map(|&(layer, x)| quad(layer, x))
            .collect();
        let mut out = Vec::new();
        let solid = split_solid_cutout(&input, &flags, &mut out);
        assert_eq!(solid, 3);
        assert_eq!(out.len(), input.len());
        let xs: Vec<f32> = out.chunks(4).map(|q| q[0].position[0]).collect();
        assert_eq!(xs, [0.0, 2.0, 4.0, 1.0, 3.0]);
    }

    #[test]
    fn an_unknown_layer_is_cutout() {
        let input: Vec<Vertex> = quad(9, 0.0).to_vec();
        let mut out = Vec::new();
        assert_eq!(split_solid_cutout(&input, &[false], &mut out), 0);
    }

    #[test]
    fn a_layer_with_a_hole_is_cutout_and_a_full_one_is_solid() {
        let mut pixels = vec![255_u8; 2 * 16];
        // Layer 1 (second 16 bytes): one texel with alpha 127 < 128.
        pixels[16 + 7] = 127;
        // Layer 0: alpha exactly at the threshold stays solid.
        pixels[3] = 128;
        assert_eq!(cutout_layers(&pixels, 16), [false, true]);
        assert!(cutout_layers(&pixels, 0).is_empty());
    }

    fn spans_of(counts: [u32; 3]) -> Vec<VertexSpan> {
        let mut start = 0;
        let mut spans: Vec<VertexSpan> = counts
            .iter()
            .map(|quads| {
                let span = VertexSpan {
                    start,
                    len: quads * 4,
                };
                start += quads * 4;
                span
            })
            .collect();
        spans.resize(SECTIONS, VertexSpan { start, len: 0 });
        spans
    }

    #[test]
    fn sections_are_split_solid_first_then_cutout_each_in_section_order() {
        let flags = [false, true];
        // Section 0: solid, cutout. Section 1: cutout. Section 2: solid, solid.
        let layers = [0, 1, 1, 0, 0];
        let input: Vec<Vertex> = layers
            .iter()
            .enumerate()
            .flat_map(|(i, &layer)| quad(layer, i as f32))
            .collect();
        let spans = spans_of([2, 1, 2]);
        let mut out = Vec::new();
        let (solid, sections) = split_sections(&input, &spans, &flags, &mut out);
        assert_eq!(solid, 3);
        let xs: Vec<f32> = out.chunks(4).map(|q| q[0].position[0]).collect();
        assert_eq!(xs, [0.0, 3.0, 4.0, 1.0, 2.0]);
        // Every test quad faces +Y (bucket 2).
        let up = 2;
        assert_eq!(
            sections[0].solid_facings[up],
            QuadRange { start: 0, len: 1 }
        );
        assert_eq!(
            sections[2].solid_facings[up],
            QuadRange { start: 1, len: 2 }
        );
        assert_eq!(
            sections[0].cutout_facings[up],
            QuadRange { start: 3, len: 1 }
        );
        assert_eq!(
            sections[1].cutout_facings[up],
            QuadRange { start: 4, len: 1 }
        );
        assert!(sections.iter().all(|section| section.sorted));
    }

    fn facing_quad(normal: [f32; 3], x: f32) -> [Vertex; 4] {
        quad(0, x).map(|vertex| Vertex { normal, ..vertex })
    }

    #[test]
    fn a_bucket_is_contiguous_over_the_column_and_hidden_facings_are_skipped() {
        // Section 0: +X, -X. Section 1: +X, +Y.
        let input: Vec<Vertex> = [
            facing_quad([1.0, 0.0, 0.0], 0.0),
            facing_quad([-1.0, 0.0, 0.0], 1.0),
            facing_quad([1.0, 0.0, 0.0], 2.0),
            facing_quad([0.0, 1.0, 0.0], 3.0),
        ]
        .concat();
        let mut out = Vec::new();
        let (_, sections) = split_sections(&input, &spans_of([2, 2, 0]), &[false], &mut out);
        let xs: Vec<f32> = out.chunks(4).map(|q| q[0].position[0]).collect();
        assert_eq!(
            xs,
            [0.0, 2.0, 1.0, 3.0],
            "+X of both sections, then -X, then +Y"
        );
        let runs = |facings: &dyn Fn(usize) -> u8| {
            let mut runs = Vec::new();
            collect_runs(&sections, (0xFFFF, facings), Group::Solid, &mut |run| {
                runs.push(run)
            });
            runs
        };
        assert_eq!(
            runs(&|_| ALL_FACINGS),
            vec![0_u32..24],
            "all visible: one draw"
        );
        // An eye west of the column sees no +X face.
        let west = |_| visible_facings([-5.0, 8.0, 8.0]);
        assert_eq!(runs(&west), vec![12_u32..24]);
    }

    #[test]
    fn the_eye_sees_the_facings_on_its_side_of_the_section() {
        let bits = |eye| visible_facings(eye);
        // Inside the box every facing can be seen.
        assert_eq!(bits([8.0, 8.0, 8.0]), ALL_FACINGS);
        // Above and to the east: no -Y (bottom) and no -X faces face it.
        let above_east = bits([20.0, 30.0, 8.0]);
        assert_eq!(above_east & (1 << 1), 0, "-X");
        assert_eq!(above_east & (1 << 3), 0, "-Y");
        assert_ne!(above_east & (1 << 0), 0, "+X");
        assert_ne!(above_east & (1 << 2), 0, "+Y");
        // On a face plane the faces in that plane are edge-on: skipped.
        assert_eq!(bits([0.0, 8.0, 8.0]) & 1, 0);
        assert_ne!(bits([0.0, 8.0, 8.0]) & (1 << 6), 0, "unaligned: always");
    }

    #[test]
    fn the_buckets_follow_the_normal_axis() {
        assert_eq!(facing([1.0, 0.0, 0.0]), 0);
        assert_eq!(facing([0.0, 0.0, -1.0]), 5);
        assert_eq!(facing([0.707, 0.0, 0.707]), UNALIGNED);
    }

    fn all_runs(sections: &[SectionQuads; SECTIONS], mask: u16) -> Vec<Range<u32>> {
        let mut runs = Vec::new();
        collect_runs(
            sections,
            (mask, &|_| ALL_FACINGS),
            Group::Solid,
            &mut |run| runs.push(run),
        );
        runs
    }

    #[test]
    fn touching_visible_sections_merge_into_one_run() {
        let mut sections = Box::new([SectionQuads::default(); SECTIONS]);
        sections[0].solid = QuadRange { start: 0, len: 2 };
        sections[1].solid = QuadRange { start: 2, len: 3 };
        sections[3].solid = QuadRange { start: 5, len: 1 };
        // Section 2 is empty: it must not split 1 and 3.
        let runs = all_runs(&sections, 0xFFFF);
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0], 0..36);
    }

    #[test]
    fn a_hidden_section_splits_the_runs() {
        let mut sections = Box::new([SectionQuads::default(); SECTIONS]);
        sections[0].solid = QuadRange { start: 0, len: 2 };
        sections[1].solid = QuadRange { start: 2, len: 3 };
        sections[2].solid = QuadRange { start: 5, len: 1 };
        assert_eq!(all_runs(&sections, 0b101), [0..12, 30..36]);
        assert!(all_runs(&sections, 0).is_empty());
    }

    #[test]
    fn spans_must_tile_the_vertices() {
        assert!(spans_tile(&spans_of([2, 1, 2]), 20));
        assert!(!spans_tile(&spans_of([2, 1, 2]), 24));
        assert!(!spans_tile(&spans_of([2, 1, 2])[..3], 20));
    }

    #[test]
    fn the_pattern_is_six_indices_per_quad() {
        assert_eq!(quad_index_pattern(2), [0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7]);
    }
}
