// SPDX-License-Identifier: MIT OR Apache-2.0
//! The memory display: how much memory the client uses.

use std::fmt::Write;

use super::hud_style::hud_settings;
use super::parse_color;
use crate::context::ModContext;
use crate::hud::{Anchor, HudContent, HudPlacement};
use crate::id::{ModCategory, ModId};
use crate::meta::ModMeta;
use crate::settings::{ModSettings, SettingDef, SettingKind};
use crate::traits::{BuiltinMod, HudSpec};

static META: ModMeta = ModMeta {
    id: ModId::new("memory"),
    name_key: "cindral.mods.memory.name",
    description_key: "cindral.mods.memory.desc",
    category: ModCategory::Hud,
    default_enabled: false,
};

const SETTINGS: &[SettingDef] = hud_settings![SettingDef {
    key: "color",
    label_key: "cindral.mods.memory.color",
    kind: SettingKind::Color { default: "#ffffff" },
}];

/// The memory mod.
pub(super) struct Memory;

impl BuiltinMod for Memory {
    fn meta(&self) -> &'static ModMeta {
        &META
    }

    fn settings(&self) -> &'static [SettingDef] {
        SETTINGS
    }

    fn hud(&self) -> Option<HudSpec> {
        Some(HudSpec {
            default_placement: HudPlacement::at(Anchor::TopRight, -4.0, 16.0),
        })
    }

    fn draw(&self, ctx: &ModContext, settings: &ModSettings, out: &mut HudContent) {
        if let Some(megabytes) = ctx.memory_mb {
            let _ = write!(
                out.line(parse_color(settings.text("color"))),
                "{megabytes} MB"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mods::drawn_texts;

    #[test]
    fn shows_the_megabytes_used() {
        let ctx = ModContext {
            memory_mb: Some(412),
            ..ModContext::default()
        };
        let texts = drawn_texts(&Memory, &ctx, &ModSettings::defaults(SETTINGS));
        assert_eq!(texts, ["412 MB"]);
    }
}
