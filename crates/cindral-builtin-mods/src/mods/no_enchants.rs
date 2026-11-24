// SPDX-License-Identifier: MIT OR Apache-2.0
//! No Enchants: no enchantment glint on swords, armour, books and potions,
//! in the hand, on players, on the ground and in the inventory. The shimmer
//! hides the colour of a pack's swords and armour in a fight, and drawing it
//! is an extra pass over every enchanted thing on screen.
//!
//! The mod holds nothing but its switch: the renderer skips the glint
//! (`Renderer::set_glint`), which the client tells each frame.

use crate::id::{ModCategory, ModId};
use crate::meta::ModMeta;
use crate::traits::BuiltinMod;

static META: ModMeta = ModMeta {
    id: ModId::new("no_enchants"),
    name_key: "cindral.mods.no_enchants.name",
    description_key: "cindral.mods.no_enchants.desc",
    category: ModCategory::Visual,
    default_enabled: false,
};

/// The no enchants mod.
pub(super) struct NoEnchants;

impl BuiltinMod for NoEnchants {
    fn meta(&self) -> &'static ModMeta {
        &META
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_is_a_visual_switch_that_starts_off() {
        assert_eq!(NoEnchants.meta().category, ModCategory::Visual);
        assert!(!NoEnchants.meta().default_enabled);
        assert!(NoEnchants.settings().is_empty());
        assert!(NoEnchants.hud().is_none());
    }
}
