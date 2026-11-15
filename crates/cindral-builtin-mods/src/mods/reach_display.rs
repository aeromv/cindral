// SPDX-License-Identifier: MIT OR Apache-2.0
//! The reach of the last hit: the distance from the eye to the point the
//! attack ray entered the target's box, as the PvP clients show it.
//!
//! Only a hit the server confirmed counts (the target was hurt right after
//! the attack), so a swing at an entity that was still invulnerable does not
//! replace the last real reach.

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
    id: ModId::new("reach_display"),
    name_key: "cindral.mods.reach_display.name",
    description_key: "cindral.mods.reach_display.desc",
    category: ModCategory::Hud,
    default_enabled: false,
};

const SETTINGS: &[SettingDef] = hud_settings![SettingDef {
    key: "color",
    label_key: "cindral.mods.reach_display.color",
    kind: SettingKind::Color { default: "#ffffff" },
},];

/// How long the last reach stays before the line goes back to "no hits".
const SHOWN_MS: u64 = 3000;

/// The reach display mod.
#[derive(Default)]
pub(super) struct ReachDisplay {
    /// The confirmed hits seen at the previous frame.
    seen_hits: u32,
    /// The reach of the last confirmed hit, in hundredths of a block.
    reach_cm: Option<u16>,
    /// When that hit was seen.
    hit_ms: u64,
    /// Whether the first frame was seen (the counts start mid-session).
    started: bool,
}

impl BuiltinMod for ReachDisplay {
    fn meta(&self) -> &'static ModMeta {
        &META
    }

    fn settings(&self) -> &'static [SettingDef] {
        SETTINGS
    }

    fn hud(&self) -> Option<HudSpec> {
        Some(HudSpec {
            default_placement: HudPlacement::at(Anchor::TopLeft, 4.0, 100.0),
        })
    }

    fn update(&mut self, ctx: &ModContext, _settings: &ModSettings) {
        let hits = ctx.combat.hits;
        if self.started && hits != self.seen_hits {
            self.reach_cm = ctx.combat.last_reach_cm;
            self.hit_ms = ctx.now_ms;
        }
        self.seen_hits = hits;
        self.started = true;
    }

    fn draw(&self, ctx: &ModContext, settings: &ModSettings, out: &mut HudContent) {
        let line = out.line(parse_color(settings.text("color")));
        let recent = ctx.now_ms.saturating_sub(self.hit_ms) < SHOWN_MS;
        // Writing into a `String` cannot fail.
        let _ = match self.reach_cm.filter(|_| recent) {
            Some(cm) => write!(line, "{}.{:02} blocks", cm / 100, cm % 100),
            None => write!(line, "No hits"),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::CombatInfo;
    use crate::mods::drawn_texts;

    fn frame(m: &mut ReachDisplay, now_ms: u64, hits: u32, reach_cm: Option<u16>) -> String {
        let ctx = ModContext {
            now_ms,
            combat: CombatInfo {
                hits,
                last_reach_cm: reach_cm,
                hurts: 0,
            },
            ..ModContext::default()
        };
        let settings = ModSettings::defaults(SETTINGS);
        m.update(&ctx, &settings);
        drawn_texts(m, &ctx, &settings).remove(0)
    }

    #[test]
    fn a_confirmed_hit_shows_its_reach() {
        let mut m = ReachDisplay::default();
        assert_eq!(frame(&mut m, 0, 0, None), "No hits");
        assert_eq!(frame(&mut m, 100, 1, Some(312)), "3.12 blocks");
        assert_eq!(frame(&mut m, 200, 1, Some(312)), "3.12 blocks");
    }

    #[test]
    fn a_swing_without_a_confirmed_hit_keeps_the_last_reach() {
        let mut m = ReachDisplay::default();
        frame(&mut m, 0, 4, None);
        assert_eq!(frame(&mut m, 100, 5, Some(287)), "2.87 blocks");
        // A new attack moved the reach but the server did not confirm it.
        assert_eq!(frame(&mut m, 200, 5, Some(301)), "2.87 blocks");
    }

    #[test]
    fn the_counts_of_a_running_session_are_not_a_hit() {
        let mut m = ReachDisplay::default();
        assert_eq!(frame(&mut m, 0, 12, Some(250)), "No hits");
    }

    #[test]
    fn the_reach_fades_after_three_seconds() {
        let mut m = ReachDisplay::default();
        frame(&mut m, 0, 0, None);
        frame(&mut m, 100, 1, Some(305));
        assert_eq!(frame(&mut m, 3099, 1, Some(305)), "3.05 blocks");
        assert_eq!(frame(&mut m, 3100, 1, Some(305)), "No hits");
    }
}
