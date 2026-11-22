// SPDX-License-Identifier: MIT OR Apache-2.0
//! The item counter: how many splash potions, golden apples, ender pearls and
//! arrows are left in the inventory, the counts a PvP fight is decided on.

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
    id: ModId::new("item_counter"),
    name_key: "cindral.mods.item_counter.name",
    description_key: "cindral.mods.item_counter.desc",
    category: ModCategory::Hud,
    default_enabled: false,
};

const SETTINGS: &[SettingDef] = hud_settings![
    SettingDef {
        key: "color",
        label_key: "cindral.mods.item_counter.color",
        kind: SettingKind::Color { default: "#ffffff" },
    },
    SettingDef {
        key: "potions",
        label_key: "cindral.mods.item_counter.potions",
        kind: SettingKind::Toggle { default: true },
    },
    SettingDef {
        key: "golden_apples",
        label_key: "cindral.mods.item_counter.golden_apples",
        kind: SettingKind::Toggle { default: true },
    },
    SettingDef {
        key: "ender_pearls",
        label_key: "cindral.mods.item_counter.ender_pearls",
        kind: SettingKind::Toggle { default: true },
    },
    SettingDef {
        key: "arrows",
        label_key: "cindral.mods.item_counter.arrows",
        kind: SettingKind::Toggle { default: true },
    },
    SettingDef {
        key: "hide_zero",
        label_key: "cindral.mods.item_counter.hide_zero",
        kind: SettingKind::Toggle { default: true },
    },
];

/// The item counter mod.
pub(super) struct ItemCounter;

impl BuiltinMod for ItemCounter {
    fn meta(&self) -> &'static ModMeta {
        &META
    }

    fn settings(&self) -> &'static [SettingDef] {
        SETTINGS
    }

    fn hud(&self) -> Option<HudSpec> {
        Some(HudSpec {
            default_placement: HudPlacement::at(Anchor::BottomRight, -4.0, -40.0),
        })
    }

    fn draw(&self, ctx: &ModContext, settings: &ModSettings, out: &mut HudContent) {
        let color = parse_color(settings.text("color"));
        let counts = ctx.items;
        let rows = [
            ("potions", "Potions", counts.splash_potions),
            ("golden_apples", "Gapples", counts.golden_apples),
            ("ender_pearls", "Pearls", counts.ender_pearls),
            ("arrows", "Arrows", counts.arrows),
        ];
        for (key, name, count) in rows {
            if !settings.bool(key) || (count == 0 && settings.bool("hide_zero")) {
                continue;
            }
            let _ = write!(out.line(color), "{name}: {count}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::ItemCounts;
    use crate::mods::drawn_texts;

    #[test]
    fn counts_the_items_and_hides_the_empty_ones() {
        let ctx = ModContext {
            items: ItemCounts {
                splash_potions: 14,
                golden_apples: 0,
                ender_pearls: 3,
                arrows: 0,
            },
            ..ModContext::default()
        };
        let texts = drawn_texts(&ItemCounter, &ctx, &ModSettings::defaults(SETTINGS));
        assert_eq!(texts, ["Potions: 14", "Pearls: 3"]);
    }
}
