// SPDX-License-Identifier: MIT OR Apache-2.0

//! Turns the widgets of the built-in mods into what the HUD renderer draws
//! (ADR-0065): measures each one, resolves its placement on the screen and hands
//! over a [`ModWidgetView`].

use umicraft_builtin_mods::{HudIcon, HudStyle, HudWidget, Shadow};
use umicraft_render::{
    gui_scale, LineIcon, ModCell, ModLine, ModWidgetView, ShadowStyle, WidgetStyle,
};
use umicraft_ui::{FontAtlas, SlotItem};

/// The texture of the status icons (`InventoryEffectRenderer`).
const STATUS_SHEET: &str = "container/inventory";

/// The status icon of each 1.8 potion, by id - 1, as `(column, row)` of the
/// 18 x 18 grid that starts at `(0, 198)` of `inventory.png`
/// (`Potion.setIconIndex`); `None` for the instant ones and Saturation.
const STATUS_ICONS: [Option<(u32, u32)>; 23] = [
    Some((0, 0)), // speed
    Some((1, 0)), // slowness
    Some((2, 0)), // haste
    Some((3, 0)), // mining fatigue
    Some((4, 0)), // strength
    None,         // instant health
    None,         // instant damage
    Some((2, 1)), // jump boost
    Some((3, 1)), // nausea
    Some((7, 0)), // regeneration
    Some((6, 1)), // resistance
    Some((7, 1)), // fire resistance
    Some((0, 2)), // water breathing
    Some((0, 1)), // invisibility
    Some((5, 1)), // blindness
    Some((4, 1)), // night vision
    Some((1, 1)), // hunger
    Some((5, 0)), // weakness
    Some((6, 0)), // poison
    Some((1, 2)), // wither
    Some((7, 2)), // health boost
    Some((2, 2)), // absorption
    None,         // saturation
];

/// What the window can draw for the pictures the mods ask for: the item
/// icons of the worn armour and the held item, and the status icon sheet.
#[derive(Debug, Clone, Default)]
pub struct WidgetIcons {
    /// The worn armour, helmet first.
    pub armor: [Option<SlotItem>; 4],
    /// The item in the selected hotbar slot.
    pub held: Option<SlotItem>,
    /// `container/inventory` in the GUI atlas, when the textures are loaded.
    pub status_sheet: Option<umicraft_assets::GuiRegion>,
}

impl WidgetIcons {
    /// Picks the status sheet out of `atlas`.
    #[must_use]
    pub fn with_atlas(mut self, atlas: Option<&umicraft_assets::GuiAtlas>) -> Self {
        self.status_sheet = atlas.and_then(|atlas| atlas.region(STATUS_SHEET)).copied();
        self
    }

    /// The picture for `icon`, or `None` when it cannot be drawn (no
    /// textures, an empty slot, an effect without an icon): the line then
    /// shows its text alone.
    #[must_use]
    pub fn resolve(&self, icon: HudIcon) -> Option<LineIcon> {
        let item = |slot: &Option<SlotItem>| {
            slot.as_ref()
                .filter(|item| item.key != u64::MAX)
                .map(|item| LineIcon::Item {
                    key: item.key,
                    glint: item.glint,
                })
        };
        match icon {
            HudIcon::None => None,
            HudIcon::Armor(slot) => item(self.armor.get(usize::from(slot))?),
            HudIcon::Held => item(&self.held),
            HudIcon::Effect(id) => {
                let (column, row) = (*STATUS_ICONS.get(usize::from(id).checked_sub(1)?)?)?;
                let sheet = self.status_sheet?;
                Some(LineIcon::Image {
                    src: sheet.sub_region(column * 18, 198 + row * 18, 18, 18),
                })
            }
        }
    }
}

/// The look the renderer draws for the style a mod chose.
fn widget_style(style: &HudStyle) -> WidgetStyle {
    let channel = |value: u32, shift: u32| ((value >> shift) & 0xFF) as f32 / 255.0;
    WidgetStyle {
        background: style
            .background
            .map(|(rgb, opacity)| [channel(rgb, 16), channel(rgb, 8), channel(rgb, 0), opacity]),
        radius: style.radius,
        padding: style.padding,
        mirrored: false,
        shadow: match style.shadow {
            Shadow::Vanilla => ShadowStyle::Vanilla,
            Shadow::Color(rgb) => ShadowStyle::Color(rgb),
            Shadow::Off => ShadowStyle::Off,
        },
    }
}

/// The views of `widgets` on a window of `display` pixels.
///
/// The placement is resolved in GUI units (window size divided by the GUI scale),
/// so a widget keeps its place when the window or the GUI scale changes. A widget
/// with no line yet (first frame) is skipped.
#[must_use]
pub fn mod_widget_views<'a>(
    widgets: impl Iterator<Item = HudWidget<'a>>,
    icons: &WidgetIcons,
    font: &FontAtlas,
    display: (u32, u32),
) -> Vec<ModWidgetView> {
    let scale = gui_scale(display.0, display.1) as f32;
    let screen = (display.0 as f32 / scale, display.1 as f32 / scale);
    widgets
        .filter(|widget| !widget.content.is_empty())
        .map(|widget| {
            let lines: Vec<ModLine> = widget
                .content
                .lines()
                .iter()
                .map(|line| ModLine {
                    icon: icons.resolve(line.icon),
                    detail: (!line.detail.is_empty())
                        .then(|| (line.detail.clone(), line.detail_color)),
                    ..ModLine::new(line.text.as_str(), line.color)
                })
                .collect();
            let cells: Vec<ModCell> = widget
                .content
                .cells()
                .iter()
                .map(|cell| ModCell {
                    rect: cell.rect,
                    text: cell.text.clone(),
                    pressed: cell.pressed,
                    color: cell.color,
                })
                .collect();
            let style = widget_style(&widget.content.style);
            let mut view = ModWidgetView {
                lines,
                cells,
                position: [0.0, 0.0],
                // Measured at scale 1: `HudPlacement::rect` applies the widget scale.
                scale: 1.0,
                style,
            };
            let rect = widget.placement.rect(view.size(font), screen);
            view.position = [rect.x, rect.y];
            // On the right half, icons go against the right edge (and the text
            // before them), as the PvP clients mirror their widgets.
            view.style.mirrored = rect.x + rect.w * 0.5 > screen.0 * 0.5;
            view.scale = widget.placement.scale;
            view
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use umicraft_builtin_mods::{mods, Anchor, HudPlacement, ModContext, ModRegistry};

    use super::*;

    fn views(placement: HudPlacement) -> Vec<ModWidgetView> {
        let mut registry = ModRegistry::new(mods::all()).expect("distinct ids");
        registry
            .set_placement("fps", placement)
            .expect("fps exists");
        registry.update(&ModContext {
            fps: 60,
            ..ModContext::default()
        });
        mod_widget_views(
            registry.hud_widgets(),
            &WidgetIcons::default(),
            &FontAtlas::fallback(),
            (800, 600),
        )
    }

    #[test]
    fn the_fps_widget_is_drawn_with_its_text() {
        let views = views(HudPlacement::default());
        assert_eq!(views.len(), 1, "only the fps mod is on by default");
        assert_eq!(views[0].lines[0].text, "60 FPS");
    }

    #[test]
    fn the_widget_scale_is_not_counted_twice() {
        // Regression guard: measuring at the widget scale AND letting
        // `HudPlacement::rect` scale again would push a bottom-right widget off its corner.
        let placement = HudPlacement {
            scale: 2.0,
            ..HudPlacement::at(Anchor::BottomRight, 0.0, 0.0)
        };
        let view = &views(placement)[0];
        let gui = f32::from(u16::try_from(gui_scale(800, 600)).unwrap_or(1));
        let (w, h) = view.size(&FontAtlas::fallback());
        assert!((view.position[0] + w - 800.0 / gui).abs() < 0.01);
        assert!((view.position[1] + h - 600.0 / gui).abs() < 0.01);
    }

    #[test]
    fn the_options_of_the_mod_reach_the_view() {
        use umicraft_builtin_mods::SettingValue;

        let mut registry = ModRegistry::new(mods::all()).expect("distinct ids");
        for (key, value) in [
            ("background_color", SettingValue::Text("#ff0000".into())),
            ("background_opacity", SettingValue::Number(0.5)),
            ("padding", SettingValue::Number(4.0)),
            ("shadow", SettingValue::Text("off".into())),
        ] {
            registry
                .set_setting("fps", key, &value)
                .expect("fps option");
        }
        registry.update(&ModContext::default());
        let view = &mod_widget_views(
            registry.hud_widgets(),
            &WidgetIcons::default(),
            &FontAtlas::fallback(),
            (800, 600),
        )[0];
        assert_eq!(view.style.background, Some([1.0, 0.0, 0.0, 0.5]));
        assert_eq!(view.style.padding, 4.0);
        assert_eq!(view.style.shadow, ShadowStyle::Off);
    }
}
