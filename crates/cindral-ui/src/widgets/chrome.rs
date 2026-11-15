// SPDX-License-Identifier: MIT OR Apache-2.0

//! The Cindral look of the vanilla-shaped widgets and menu backgrounds.
//!
//! With [`Ui::set_modern_chrome`] on, buttons, sliders and the menu
//! backgrounds are drawn as flat rounded shapes in the theme colours instead
//! of the `widgets.png` and dirt textures; the layout, the hit areas and the
//! Minecraft font stay the same, so every screen keeps its vanilla behaviour.
//! With it off (the "Classic" interface style) the textures of the resource
//! pack are used, as in vanilla.

use crate::context::Ui;
use crate::draw::Color;
use crate::geom::Rect;
use crate::modern::theme;

/// A button at rest, under the pointer, and one that cannot be pressed.
const BUTTON: Color = [0.075, 0.075, 0.085, 0.82];
const BUTTON_HOVER: Color = [0.15, 0.13, 0.12, 0.92];
const BUTTON_DISABLED: Color = [0.06, 0.06, 0.07, 0.55];
/// The hairline around a button.
const RIM: Color = [1.0, 1.0, 1.0, 0.07];

/// Label colours of a button: at rest, under the pointer, disabled.
pub(crate) const LABEL: Color = [0.90, 0.90, 0.91, 1.0];
pub(crate) const LABEL_HOVER: Color = [1.0, 0.86, 0.72, 1.0];
pub(crate) const LABEL_DISABLED: Color = [0.46, 0.46, 0.49, 1.0];

/// Corner radius of buttons and sliders, in GUI units.
const RADIUS: f32 = 2.0;

/// The state a button is drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ButtonLook {
    Rest,
    Hover,
    Disabled,
}

/// Draws the body of a button: a rounded fill with a hairline rim, and an
/// ember line along the bottom while it is under the pointer. `scale` is the
/// pixels per GUI unit.
pub(crate) fn button_body(ui: &mut Ui, rect: Rect, scale: f32, look: ButtonLook) {
    let radius = RADIUS * scale;
    let fill = match look {
        ButtonLook::Rest => BUTTON,
        ButtonLook::Hover => BUTTON_HOVER,
        ButtonLook::Disabled => BUTTON_DISABLED,
    };
    ui.draw().rounded_rect(rect, radius, RIM);
    let inner = inset(rect, scale.max(1.0) * 0.5);
    ui.draw().rounded_rect(inner, radius, fill);
    if look == ButtonLook::Hover {
        let line = (scale * 0.5).max(1.0);
        let accent = Rect::new(
            rect.x + radius,
            rect.bottom() - line - scale * 0.5,
            (rect.width - 2.0 * radius).max(0.0),
            line,
        );
        ui.draw().rect(accent, theme::ACCENT);
    }
}

/// Draws a slider track with the part left of the handle tinted, and the
/// handle as a rounded ember block. `handle` is the handle rectangle.
pub(crate) fn slider_body(ui: &mut Ui, rect: Rect, handle: Rect, scale: f32, active: bool) {
    let radius = RADIUS * scale;
    ui.draw().rounded_rect(rect, radius, RIM);
    let inner = inset(rect, scale.max(1.0) * 0.5);
    ui.draw().rounded_rect(inner, radius, BUTTON);
    let filled = Rect::new(
        inner.x,
        inner.y,
        (handle.x + handle.width * 0.5 - inner.x).clamp(0.0, inner.width),
        inner.height,
    );
    ui.draw()
        .rounded_rect(filled, radius, theme::with_opacity(theme::ACCENT, 0.12));
    let knob = inset(handle, scale);
    let color = if active {
        theme::ACCENT_BRIGHT
    } else {
        theme::ACCENT
    };
    ui.draw().rounded_rect(knob, radius, color);
}

/// Fills `viewport` with the background of a menu with no world behind it:
/// a deep vertical gradient with a faint ember glow rising from the bottom.
pub(crate) fn menu_background(ui: &mut Ui, viewport: Rect) {
    ui.draw()
        .vertical_gradient(viewport, theme::BACKDROP_TOP, theme::BACKDROP_BOTTOM);
    let glow_height = viewport.height * 0.45;
    let glow = Rect::new(
        viewport.x,
        viewport.bottom() - glow_height,
        viewport.width,
        glow_height,
    );
    ui.draw().vertical_gradient(
        glow,
        theme::with_opacity(theme::ACCENT, 0.0),
        theme::with_opacity(theme::ACCENT, 0.07),
    );
}

/// The colour of the menu background gradient at height `y` of `viewport`.
fn backdrop_at(viewport: Rect, y: f32) -> Color {
    let t = if viewport.height > 0.0 {
        (y - viewport.y) / viewport.height
    } else {
        0.0
    };
    theme::mix(theme::BACKDROP_TOP, theme::BACKDROP_BOTTOM, t)
}

/// Fills the part `band` of `viewport` with the background, so a band drawn
/// over a scrolled list matches the gradient behind it.
pub(crate) fn menu_background_band(ui: &mut Ui, viewport: Rect, band: Rect) {
    if band.is_empty() {
        return;
    }
    let top = backdrop_at(viewport, band.y);
    let bottom = backdrop_at(viewport, band.bottom());
    ui.draw().vertical_gradient(band, top, bottom);
}

/// The container of a scrolling list: a dark sheet over the background.
pub(crate) fn list_background(ui: &mut Ui, list: Rect) {
    ui.draw().rect(list, [0.0, 0.0, 0.0, 0.32]);
}

/// The hairlines along the top and bottom edges of a list.
pub(crate) fn list_edges(ui: &mut Ui, list: Rect, scale: f32) {
    let line = scale.max(1.0) * 0.5;
    let color = [1.0, 1.0, 1.0, 0.08];
    ui.draw()
        .rect(Rect::new(list.x, list.y, list.width, line), color);
    ui.draw().rect(
        Rect::new(list.x, list.bottom() - line, list.width, line),
        color,
    );
}

/// `rect` shrunk by `by` on every side.
fn inset(rect: Rect, by: f32) -> Rect {
    Rect::new(
        rect.x + by,
        rect.y + by,
        (rect.width - 2.0 * by).max(0.0),
        (rect.height - 2.0 * by).max(0.0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::DrawCommand;
    use crate::input::UiInput;

    fn commands(f: impl FnOnce(&mut Ui)) -> Vec<DrawCommand> {
        let viewport = Rect::new(0.0, 0.0, 320.0, 240.0);
        let mut ui = Ui::new(viewport);
        ui.begin_frame(viewport, UiInput::default());
        f(&mut ui);
        ui.draw().commands().to_vec()
    }

    #[test]
    fn a_hovered_button_gets_the_ember_line() {
        let rect = Rect::new(10.0, 10.0, 100.0, 20.0);
        let rest = commands(|ui| button_body(ui, rect, 2.0, ButtonLook::Rest));
        let hover = commands(|ui| button_body(ui, rect, 2.0, ButtonLook::Hover));
        assert_eq!(hover.len(), rest.len() + 1);
    }

    #[test]
    fn a_band_matches_the_gradient_behind_it() {
        let viewport = Rect::new(0.0, 0.0, 100.0, 200.0);
        assert_eq!(backdrop_at(viewport, 0.0), theme::BACKDROP_TOP);
        assert_eq!(backdrop_at(viewport, 200.0), theme::BACKDROP_BOTTOM);
    }
}
