// SPDX-License-Identifier: MIT OR Apache-2.0
//! How a HUD widget looks around its text: the background, the corners, the
//! margin and the shadow of the text. Every HUD mod shares these options
//! (`mods::hud_style`), so they look and behave the same in all of them.

use crate::settings::ModSettings;

/// The shadow of the text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Shadow {
    /// A quarter of the brightness of the text, as `drawStringWithShadow` does.
    #[default]
    Vanilla,
    /// A fixed color, `0xRRGGBB`.
    Color(u32),
    /// No shadow.
    Off,
}

/// The look of one widget.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudStyle {
    /// The background as `(0xRRGGBB, opacity)`, or `None` for no background.
    pub background: Option<(u32, f32)>,
    /// The corner radius of the background, in GUI units.
    pub radius: f32,
    /// The margin between the background and the text, in GUI units.
    pub padding: f32,
    /// The outline around the background as `(0xRRGGBB, opacity)`, or `None`.
    pub border: Option<(u32, f32)>,
    /// The shadow of the text.
    pub shadow: Shadow,
}

impl Default for HudStyle {
    /// A black panel at 40 % with soft corners and a faint white outline.
    fn default() -> Self {
        Self {
            background: Some((0x00_0000, 0.4)),
            radius: 3.0,
            padding: 3.0,
            border: Some((0xFF_FFFF, 0.15)),
            shadow: Shadow::Vanilla,
        }
    }
}

impl HudStyle {
    /// Reads the shared options of `settings`.
    #[must_use]
    pub fn from_settings(settings: &ModSettings) -> Self {
        let background = settings.bool("background").then(|| {
            (
                crate::mods::parse_color(settings.text("background_color")),
                settings.number("background_opacity"),
            )
        });
        // An outline goes with a panel: alone it would frame empty air.
        let border = (background.is_some() && settings.bool("border")).then(|| {
            (
                crate::mods::parse_color(settings.text("border_color")),
                settings.number("border_opacity"),
            )
        });
        let shadow = match settings.text("shadow") {
            "off" => Shadow::Off,
            "custom" => Shadow::Color(crate::mods::parse_color(settings.text("shadow_color"))),
            _ => Shadow::Vanilla,
        };
        Self {
            background,
            radius: settings.number("corner_radius"),
            padding: settings.number("padding"),
            border,
            shadow,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mods::hud_style::hud_settings;
    use crate::settings::{SettingDef, SettingValue};

    const SHARED: &[SettingDef] = hud_settings![];

    fn settings() -> ModSettings {
        ModSettings::defaults(SHARED)
    }

    #[test]
    fn the_defaults_of_the_options_are_the_default_look() {
        assert_eq!(HudStyle::from_settings(&settings()), HudStyle::default());
    }

    #[test]
    fn every_option_reaches_the_style() {
        let mut settings = settings();
        for (key, value) in [
            ("background_color", SettingValue::Text("#102030".into())),
            ("background_opacity", SettingValue::Number(0.8)),
            ("corner_radius", SettingValue::Number(4.0)),
            ("padding", SettingValue::Number(5.0)),
            ("border_color", SettingValue::Text("#ff7329".into())),
            ("border_opacity", SettingValue::Number(0.5)),
            ("shadow", SettingValue::Text("custom".into())),
            ("shadow_color", SettingValue::Text("#ff0000".into())),
        ] {
            settings.set(key, &value).expect("known key");
        }
        assert_eq!(
            HudStyle::from_settings(&settings),
            HudStyle {
                background: Some((0x10_2030, 0.8)),
                radius: 4.0,
                padding: 5.0,
                border: Some((0xff_7329, 0.5)),
                shadow: Shadow::Color(0xff_0000),
            }
        );
    }

    #[test]
    fn the_background_and_the_shadow_can_be_turned_off() {
        let mut settings = settings();
        settings
            .set("background", &SettingValue::Bool(false))
            .expect("known key");
        settings
            .set("shadow", &SettingValue::Text("off".into()))
            .expect("known key");
        let style = HudStyle::from_settings(&settings);
        assert_eq!(style.background, None);
        assert_eq!(style.border, None, "no outline without a panel");
        assert_eq!(style.shadow, Shadow::Off);
    }
}
