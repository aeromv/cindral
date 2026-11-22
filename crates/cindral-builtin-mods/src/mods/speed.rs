// SPDX-License-Identifier: MIT OR Apache-2.0
//! The speed display: how fast the player moves over the ground, in blocks per
//! second, measured over the last half second so a jump does not make it jump.

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
    id: ModId::new("speed"),
    name_key: "cindral.mods.speed.name",
    description_key: "cindral.mods.speed.desc",
    category: ModCategory::Hud,
    default_enabled: false,
};

const SETTINGS: &[SettingDef] = hud_settings![
    SettingDef {
        key: "color",
        label_key: "cindral.mods.speed.color",
        kind: SettingKind::Color { default: "#ffffff" },
    },
    SettingDef {
        key: "vertical",
        label_key: "cindral.mods.speed.vertical",
        kind: SettingKind::Toggle { default: false },
    },
];

/// How long a measurement spans, in milliseconds.
const WINDOW_MS: u64 = 500;

/// The speed mod: the position and time at the start of the current window
/// and the last speed measured.
#[derive(Debug, Default)]
pub(super) struct Speed {
    start: Option<([f64; 3], u64)>,
    blocks_per_second: f64,
}

impl BuiltinMod for Speed {
    fn meta(&self) -> &'static ModMeta {
        &META
    }

    fn settings(&self) -> &'static [SettingDef] {
        SETTINGS
    }

    fn hud(&self) -> Option<HudSpec> {
        Some(HudSpec {
            default_placement: HudPlacement::at(Anchor::TopLeft, 4.0, 76.0),
        })
    }

    fn update(&mut self, ctx: &ModContext, settings: &ModSettings) {
        let Some((from, at)) = self.start else {
            self.start = Some((ctx.position, ctx.now_ms));
            return;
        };
        let elapsed = ctx.now_ms.saturating_sub(at);
        if elapsed < WINDOW_MS {
            return;
        }
        let [dx, dy, dz] = [0, 1, 2].map(|axis| ctx.position[axis] - from[axis]);
        let distance = if settings.bool("vertical") {
            (dx * dx + dy * dy + dz * dz).sqrt()
        } else {
            (dx * dx + dz * dz).sqrt()
        };
        // A teleport is not a speed.
        self.blocks_per_second = if distance > 50.0 {
            0.0
        } else {
            distance * 1000.0 / elapsed as f64
        };
        self.start = Some((ctx.position, ctx.now_ms));
    }

    fn draw(&self, _ctx: &ModContext, settings: &ModSettings, out: &mut HudContent) {
        let _ = write!(
            out.line(parse_color(settings.text("color"))),
            "{:.2} m/s",
            self.blocks_per_second
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mods::drawn_texts;

    fn at(x: f64, now_ms: u64) -> ModContext {
        ModContext {
            position: [x, 64.0, 0.0],
            now_ms,
            ..ModContext::default()
        }
    }

    #[test]
    fn sprinting_reads_about_five_and_a_half_blocks_a_second() {
        let settings = ModSettings::defaults(SETTINGS);
        let mut speed = Speed::default();
        speed.update(&at(0.0, 0), &settings);
        speed.update(&at(2.8, 500), &settings);
        assert_eq!(drawn_texts(&speed, &at(2.8, 500), &settings), ["5.60 m/s"]);
    }

    #[test]
    fn a_teleport_reads_as_standing_still() {
        let settings = ModSettings::defaults(SETTINGS);
        let mut speed = Speed::default();
        speed.update(&at(0.0, 0), &settings);
        speed.update(&at(500.0, 500), &settings);
        assert_eq!(
            drawn_texts(&speed, &at(500.0, 500), &settings),
            ["0.00 m/s"]
        );
    }
}
