// SPDX-License-Identifier: MIT OR Apache-2.0

//! One mod as a tile: its icon large in the middle, its name under it, a
//! small gear in the corner for its settings, and a state bar across the
//! bottom that turns it on and off. The description lives on the settings
//! page; the grid only has to be scanned. A mod that is on lights its icon
//! and its bar with the accent; the pointer lifts the tile.

use crate::context::Ui;
use crate::geom::Rect;
use crate::icons::Icon;
use crate::modern::theme::{self, mix, with_opacity};
use crate::modern::{draw_icon, draw_text, fit_text, Align};

use super::button::icon_button;
use super::motion::Motion;
use super::style::{bordered, card_colors, hover_level, Paint, MOD_NAME_SIZE, SURFACE_2};
use super::view::ModCardView;

/// Space between the card edge and its content, in GUI units.
const PAD: f32 = 6.0;
/// Size of the icon in the middle of a card.
const ICON: f32 = 18.0;
/// Edge of the gear button.
const GEAR: f32 = 12.0;
/// Height of the state bar along the bottom.
const BAR: f32 = 14.0;
/// Size of the text of the state bar.
const BAR_TEXT: f32 = 5.5;

/// What the player did on a card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CardAction {
    /// The switch was clicked; the state asked for.
    Toggle(bool),
    /// The settings button was clicked.
    OpenSettings,
}

/// The icon of the mod called `id`; a mod without a picture gets the grid.
pub(super) fn mod_icon(id: &str) -> Icon {
    match id {
        "fps" => Icon::Gauge,
        "coordinates" => Icon::Target,
        "cps" => Icon::Cursor,
        "ping" => Icon::Signal,
        "toggle_sprint" => Icon::Run,
        "motion_blur" => Icon::Blur,
        "fullbright" => Icon::Lightbulb,
        "hit_color" => Icon::Sword,
        "zoom" => Icon::Binoculars,
        "no_hurt_cam" => Icon::Camera,
        "keystrokes" => Icon::Keyboard,
        "combo_counter" => Icon::Lightning,
        "armor_status" => Icon::Shield,
        "potion_effects" => Icon::Flask,
        "legacy_animations" => Icon::Sword,
        "direction_hud" => Icon::Compass,
        "capes" => Icon::Cape,
        "reach_display" => Icon::Target,
        "speed" => Icon::Run,
        "item_counter" => Icon::Flask,
        "clock" => Icon::Compass,
        "memory" => Icon::Sliders,
        _ => Icon::Grid,
    }
}

/// The state bar and the settings button of a card at `rect`, in physical
/// pixels: the bar across the bottom, the gear in the top right corner.
pub(super) fn controls(rect: Rect, scale: f32) -> (Rect, Rect) {
    let bar = Rect::new(
        rect.x + PAD * scale,
        rect.bottom() - (PAD + BAR) * scale,
        rect.width - 2.0 * PAD * scale,
        BAR * scale,
    );
    let gear = Rect::new(
        rect.right() - (PAD + GEAR) * scale,
        rect.y + PAD * scale,
        GEAR * scale,
        GEAR * scale,
    );
    (bar, gear)
}

/// Draws the square tile of a mod in `rect`: filled with the accent and the
/// icon white while the mod is on (`on` is the level, `0.0` to `1.0`), a faint
/// square and a muted icon while it is off.
pub(super) fn icon_tile(ui: &mut Ui, paint: Paint<'_>, rect: Rect, (id, on): (&str, f32)) {
    let radius = rect.width * 0.27;
    let fill = mix(SURFACE_2, theme::ACCENT, on);
    ui.draw().rounded_rect(rect, radius, fill);
    let icon_colour = mix(with_opacity(theme::TEXT, 0.85), [1.0, 1.0, 1.0, 1.0], on);
    let centre = (rect.x + rect.width * 0.5, rect.y + rect.height * 0.5);
    draw_icon(
        ui,
        paint.brand,
        mod_icon(id),
        centre,
        (rect.width * 0.5, icon_colour),
    );
}

/// Draws `card` in `rect` and reports what was clicked. `tag` is the name of
/// the category of the mod, shown on the settings page only.
pub(super) fn card(
    ui: &mut Ui,
    paint: Paint<'_>,
    rect: Rect,
    (card, motion): (&ModCardView, &mut Motion),
    _tag: &str,
) -> Option<CardAction> {
    let scale = paint.scale;
    let hover = motion.hover(&format!("hover/{}", card.id), ui.is_hovered(rect));
    let (on, _speed) = motion.level(&format!("on/{}", card.id), card.enabled);
    let (bar, gear) = controls(rect, scale);
    let centre_x = rect.x + rect.width * 0.5;

    bordered(ui, rect, 4.0 * scale, card_colors(hover, on));

    // The icon and the name, centred in the space above the bar.
    let room_top = rect.y + PAD * scale;
    let room = bar.y - room_top;
    let name_size = MOD_NAME_SIZE * scale;
    let block = ICON * scale + 5.0 * scale + name_size;
    let icon_y = room_top + (room - block) * 0.5 + ICON * scale * 0.5;
    let icon_colour = mix(with_opacity(theme::TEXT, 0.55), theme::ACCENT, on);
    draw_icon(
        ui,
        paint.brand,
        mod_icon(&card.id),
        (centre_x, icon_y),
        (ICON * scale, icon_colour),
    );
    let name_y = icon_y + ICON * scale * 0.5 + 5.0 * scale + name_size * 0.5;
    let name = fit_text(
        paint.brand,
        &card.name,
        name_size,
        rect.width - 2.0 * PAD * scale,
    );
    paint.text(
        ui,
        &name,
        (centre_x, name_y),
        (MOD_NAME_SIZE, theme::TEXT, Align::Center),
    );

    ui.push_id(&card.id);
    let settings_id = ui.id("settings");
    let bar_id = ui.id("switch");
    let gear_fade = hover_level(ui, gear);
    let settings = icon_button(
        ui,
        paint.brand,
        (settings_id, gear),
        (Icon::Gear, scale, gear_fade),
    );
    let flipped = state_bar(ui, paint, (bar_id, bar), (card.enabled, on));
    ui.pop_id();
    flipped
        .map(CardAction::Toggle)
        .or(settings.clicked.then_some(CardAction::OpenSettings))
}

/// The bar that shows and flips the state of a mod: the accent with
/// "ENABLED" while it is on (`on` is the eased level), a dark bar with
/// "DISABLED" while it is off. Returns the state asked for on a click.
fn state_bar(
    ui: &mut Ui,
    paint: Paint<'_>,
    (id, rect): (crate::id::WidgetId, Rect),
    (enabled, on): (bool, f32),
) -> Option<bool> {
    let response = ui.response(id, rect);
    let hover = if response.hovered { 1.0 } else { 0.0 };
    let off = mix(SURFACE_2, [1.0, 1.0, 1.0, 0.10], hover);
    let lit = mix(theme::ACCENT, theme::ACCENT_BRIGHT, hover);
    ui.draw()
        .rounded_rect(rect, 3.0 * paint.scale, mix(off, lit, on));
    let label = if enabled { "ENABLED" } else { "DISABLED" };
    let colour = mix(with_opacity(theme::TEXT, 0.6), [1.0, 1.0, 1.0, 1.0], on);
    let size = BAR_TEXT * paint.scale;
    let centre = (rect.x + rect.width * 0.5, rect.y + rect.height * 0.5);
    draw_text(
        ui,
        paint.brand,
        label,
        centre,
        (size, colour, Align::Center),
    );
    if response.clicked {
        ui.note_button_press();
        return Some(!enabled);
    }
    None
}
