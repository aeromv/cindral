// SPDX-License-Identifier: MIT OR Apache-2.0
//! Hit Color: the colour a player or mob takes for the half second after a
//! hit. Vanilla blends 30 % towards pure red, which reads pale on most skins;
//! the mod starts on a deeper red, and both the colour and how strongly it
//! covers the skin can be changed.
//!
//! The renderer tints the hurt entities (`Renderer::set_hurt_color`), which
//! the client tells each frame what this mod asks for.

use crate::id::{ModCategory, ModId};
use crate::meta::ModMeta;
use crate::settings::{SettingDef, SettingKind};
use crate::traits::BuiltinMod;

static META: ModMeta = ModMeta {
    id: ModId::new("hit_color"),
    name_key: "cindral.mods.hit_color.name",
    description_key: "cindral.mods.hit_color.desc",
    category: ModCategory::Visual,
    default_enabled: true,
};

const SETTINGS: &[SettingDef] = &[
    SettingDef {
        key: "color",
        label_key: "cindral.mods.hit_color.color",
        kind: SettingKind::Color { default: "#c00000" },
    },
    SettingDef {
        key: "strength",
        label_key: "cindral.mods.hit_color.strength",
        kind: SettingKind::Slider {
            min: 0.1,
            max: 0.9,
            step: 0.05,
            default: 0.5,
        },
    },
];

/// The hit colour mod.
pub(super) struct HitColor;

impl BuiltinMod for HitColor {
    fn meta(&self) -> &'static ModMeta {
        &META
    }

    fn settings(&self) -> &'static [SettingDef] {
        SETTINGS
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::ModSettings;

    #[test]
    fn it_starts_on_with_a_deeper_red_than_vanilla() {
        assert!(HitColor.meta().default_enabled);
        let settings = ModSettings::defaults(SETTINGS);
        assert_eq!(super::super::parse_color(settings.text("color")), 0xC0_0000);
        assert!(settings.number("strength") > 0.3, "stronger than vanilla");
        assert!(HitColor.hud().is_none(), "it draws no widget");
    }
}
