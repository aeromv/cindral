// SPDX-License-Identifier: MIT OR Apache-2.0

//! Which entities are not meshed because the player cannot see them (perf 2, C1).
//!
//! Meshing an entity costs 3 to 20 us a frame (a Pentium G5400: 240 entities,
//! 4.75 ms), and the reference draws only what is in range and in the view
//! (`RenderManager.shouldRender`: `Entity.isInRangeToRenderDist`, then
//! `ICamera.isBoundingBoxInFrustum`). The rules here follow it, with margins
//! that only ever keep an entity the reference would skip, never the reverse:
//!
//! - range: the average edge of the body times 64 blocks (16 for a dropped
//!   item, 32 for an orb, 64 for a player), as `isInRangeToRenderDist`; the
//!   kinds with no body keep a 128-block range;
//! - view: a sphere around the body, one block wider for the swinging limbs and
//!   the held item, against the cone that holds the frustum.
//!
//! The list of entities that the meshing receives stays whole, because a
//! bobber, a leash or the beam of a crystal reads another entity from it.

use glam::Vec3;
use umicraft_game::combat::hitbox::size_of;
use umicraft_game::entity::{EntityKind, EntityRenderSnapshot};

/// The range of what has no body to measure, in blocks.
const MAX_DISTANCE: f32 = 128.0;
/// Inside this distance every entity is meshed (the third-person camera and
/// a body that fills the screen).
const NEAR: f32 = 4.0;
/// Extra radius around the body, in blocks: limbs swing out of the box and the
/// held item reaches further.
const LIMB_MARGIN: f32 = 1.0;
/// The radius kept for the very large ones and the bolt, which reach far.
const BIG_RADIUS: f32 = 64.0;

/// Where the player looks, in the space of the entity snapshots (relative to the
/// render origin).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CullView {
    /// The eye.
    pub eye: Vec3,
    /// The unit look direction.
    pub forward: Vec3,
    /// Half the angle of the cone that holds the frustum, in radians
    /// (`Camera::view_cone_half_angle`).
    pub half_angle: f32,
}

/// Whether the entity is large enough to keep far behind the camera.
fn is_big(kind: EntityKind) -> bool {
    matches!(
        kind,
        EntityKind::Dragon
            | EntityKind::Giant
            | EntityKind::Ghast
            | EntityKind::Wither
            | EntityKind::Slime
            | EntityKind::MagmaCube
            | EntityKind::Lightning
    )
}

/// The width and height the range and the sphere are taken from: the body
/// (`hitbox::size_of`), or the size of the small objects that have none
/// there (MCP constructors: `EntityItem` and `EntityXPOrb` setSize).
fn body(entity: &EntityRenderSnapshot) -> Option<(f32, f32)> {
    match entity.kind {
        EntityKind::Item => Some((0.25, 0.25)),
        EntityKind::XpOrb => Some((0.5, 0.5)),
        kind => size_of(kind, &entity.meta).map(|(width, height)| (width as f32, height as f32)),
    }
}

/// How far the entity is drawn, in blocks (`Entity.isInRangeToRenderDist`:
/// the average edge of the box times 64, `renderDistanceWeight` 1). The
/// hanging entities override it with 16 times 64 (`EntityHanging`), more than
/// any view: they keep the long range.
fn range(entity: &EntityRenderSnapshot) -> f32 {
    if matches!(
        entity.kind,
        EntityKind::ItemFrame | EntityKind::Painting | EntityKind::LeashKnot
    ) {
        return MAX_DISTANCE;
    }
    body(entity).map_or(MAX_DISTANCE, |(width, height)| {
        ((width + height + width) / 3.0 * 64.0).min(MAX_DISTANCE)
    })
}

/// Whether `entity` is left out of the mesh for `view`.
#[must_use]
pub fn out_of_view(entity: &EntityRenderSnapshot, view: CullView) -> bool {
    if is_big(entity.kind) {
        // The old cautious rule: only what is well behind the camera.
        return (entity.position - view.eye).dot(view.forward) < -BIG_RADIUS;
    }
    let (width, height) = body(entity).unwrap_or((1.0, 2.0));
    let centre = entity.position + Vec3::new(0.0, height * 0.5, 0.0);
    let offset = centre - view.eye;
    let distance = offset.length();
    if distance < NEAR {
        return false;
    }
    // The range is measured from the feet, as `isInRangeToRender3d` does.
    if (entity.position - view.eye).length() > range(entity) {
        return true;
    }
    let radius = 0.5 * (2.0 * width * width + height * height).sqrt() + LIMB_MARGIN;
    // The distance from the centre to the side of the cone, in the plane of the
    // axis and the centre: positive outside. It is never more than the true
    // distance to the cone, so a sphere it puts outside is outside.
    let along = offset.dot(view.forward);
    let across = (distance * distance - along * along).max(0.0).sqrt();
    let (sin, cos) = view.half_angle.min(1.55).sin_cos();
    across * cos - along * sin > radius
}

#[cfg(test)]
mod tests {
    use super::*;
    use umicraft_game::entity::EntityId;

    /// A 70-degree view at 16:9 (`Camera::view_cone_half_angle`).
    const VIEW: CullView = CullView {
        eye: Vec3::ZERO,
        forward: Vec3::NEG_Z,
        half_angle: 0.959,
    };

    fn at(kind: EntityKind, position: Vec3) -> EntityRenderSnapshot {
        EntityRenderSnapshot::new(EntityId(1), kind, position)
    }

    #[test]
    fn what_is_in_front_or_close_is_meshed() {
        let ahead = at(EntityKind::Cow, Vec3::new(0.0, 0.0, -30.0));
        let close_behind = at(EntityKind::Cow, Vec3::new(0.0, 0.0, 3.0));
        let edge_of_view = at(EntityKind::Player, Vec3::new(20.0, 0.0, -15.0));
        assert!(!out_of_view(&ahead, VIEW));
        assert!(!out_of_view(&close_behind, VIEW), "close behind");
        assert!(
            !out_of_view(&edge_of_view, VIEW),
            "inside the corner of the view"
        );
    }

    #[test]
    fn what_is_beside_or_behind_is_skipped() {
        let beside = at(EntityKind::Player, Vec3::new(30.0, 0.0, -2.0));
        let behind = at(EntityKind::Cow, Vec3::new(0.0, 0.0, 30.0));
        assert!(out_of_view(&beside, VIEW), "out of the side of the view");
        assert!(out_of_view(&behind, VIEW));
    }

    #[test]
    fn a_body_that_reaches_into_the_view_is_kept() {
        // Its centre is just outside the cone, its side inside.
        let angle = VIEW.half_angle + 0.02;
        let position = Vec3::new(angle.sin(), 0.0, -angle.cos()) * 20.0 - Vec3::Y;
        assert!(!out_of_view(&at(EntityKind::Player, position), VIEW));
    }

    #[test]
    fn the_range_follows_the_size_of_the_body() {
        let ahead = |kind, distance: f32| at(kind, Vec3::new(0.0, 0.0, -distance));
        assert!(!out_of_view(&ahead(EntityKind::Item, 15.0), VIEW));
        assert!(
            out_of_view(&ahead(EntityKind::Item, 17.0), VIEW),
            "an item at 16"
        );
        assert!(!out_of_view(&ahead(EntityKind::Player, 60.0), VIEW));
        assert!(
            out_of_view(&ahead(EntityKind::Player, 70.0), VIEW),
            "a player at 64"
        );
        assert!(
            !out_of_view(&ahead(EntityKind::Arrow, 100.0), VIEW),
            "no body: 128"
        );
        assert!(
            !out_of_view(&ahead(EntityKind::ItemFrame, 100.0), VIEW),
            "hanging"
        );
    }

    #[test]
    fn the_giants_and_the_bolt_are_kept_unless_far_behind() {
        let behind = Vec3::new(0.0, 0.0, 30.0);
        assert!(!out_of_view(&at(EntityKind::Dragon, behind), VIEW));
        assert!(!out_of_view(&at(EntityKind::Lightning, behind), VIEW));
        let far = Vec3::new(0.0, 0.0, -200.0);
        assert!(!out_of_view(&at(EntityKind::Dragon, far), VIEW));
    }
}
