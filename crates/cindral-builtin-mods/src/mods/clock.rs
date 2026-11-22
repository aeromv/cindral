// SPDX-License-Identifier: MIT OR Apache-2.0
//! The clock: the time of the computer, in 24 or 12 hours.

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
    id: ModId::new("clock"),
    name_key: "cindral.mods.clock.name",
    description_key: "cindral.mods.clock.desc",
    category: ModCategory::Hud,
    default_enabled: false,
};

const SETTINGS: &[SettingDef] = hud_settings![
    SettingDef {
        key: "color",
        label_key: "cindral.mods.clock.color",
        kind: SettingKind::Color { default: "#ffffff" },
    },
    SettingDef {
        key: "twelve_hours",
        label_key: "cindral.mods.clock.twelve_hours",
        kind: SettingKind::Toggle { default: false },
    },
    SettingDef {
        key: "seconds",
        label_key: "cindral.mods.clock.seconds",
        kind: SettingKind::Toggle { default: false },
    },
];

/// The clock mod.
pub(super) struct Clock;

impl BuiltinMod for Clock {
    fn meta(&self) -> &'static ModMeta {
        &META
    }

    fn settings(&self) -> &'static [SettingDef] {
        SETTINGS
    }

    fn hud(&self) -> Option<HudSpec> {
        Some(HudSpec {
            default_placement: HudPlacement::at(Anchor::TopRight, -4.0, 4.0),
        })
    }

    fn draw(&self, ctx: &ModContext, settings: &ModSettings, out: &mut HudContent) {
        let Some([hour, minute, second]) = ctx.local_time else {
            return;
        };
        let line = out.line(parse_color(settings.text("color")));
        if settings.bool("twelve_hours") {
            let shown = match hour % 12 {
                0 => 12,
                other => other,
            };
            let _ = write!(line, "{shown}:{minute:02}");
            if settings.bool("seconds") {
                let _ = write!(line, ":{second:02}");
            }
            line.push_str(if hour < 12 { " AM" } else { " PM" });
        } else {
            let _ = write!(line, "{hour:02}:{minute:02}");
            if settings.bool("seconds") {
                let _ = write!(line, ":{second:02}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mods::drawn_texts;

    fn texts(time: [u8; 3], twelve: bool) -> Vec<String> {
        let mut settings = ModSettings::defaults(SETTINGS);
        settings
            .set("twelve_hours", &crate::settings::SettingValue::Bool(twelve))
            .expect("a toggle");
        let ctx = ModContext {
            local_time: Some(time),
            ..ModContext::default()
        };
        drawn_texts(&Clock, &ctx, &settings)
    }

    #[test]
    fn shows_the_time_in_24_or_12_hours() {
        assert_eq!(texts([21, 5, 9], false), ["21:05"]);
        assert_eq!(texts([21, 5, 9], true), ["9:05 PM"]);
        assert_eq!(texts([0, 30, 0], true), ["12:30 AM"]);
    }
}
