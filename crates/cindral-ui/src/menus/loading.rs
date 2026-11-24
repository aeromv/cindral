// SPDX-License-Identifier: MIT OR Apache-2.0

//! The screen shown while the game starts: the logo, a bar that follows the
//! real loading steps, the step's name and the version.
//!
//! It also fades out over the first screen after it, so the start ends with
//! a short dissolve instead of a cut.

use crate::context::Ui;
use crate::draw::Color;
use crate::geom::Rect;
use crate::modern::{draw_text, theme, Align};
use crate::text::FontAtlas;

/// Edge of the logo, in GUI units.
const LOGO: f32 = 44.0;
/// Width of the bar, in GUI units; narrower on a small window.
const BAR_WIDTH: f32 = 150.0;
/// Space between the logo and the bar, in GUI units.
const GAP: f32 = 20.0;
/// Margin of the corner texts, in GUI units.
const MARGIN: f32 = 6.0;
/// A flat background, a shade darker than the menus.
const BACKGROUND: Color = [0.047, 0.044, 0.050, 1.0];
/// The empty part of the bar.
const TRACK: Color = [1.0, 1.0, 1.0, 0.09];

/// What the loading screen shows this frame.
#[derive(Debug, Clone, Copy)]
pub struct LoadingView<'a> {
    /// The filled share of the bar, `0.0..=1.0`.
    pub fraction: f32,
    /// What is being loaded ("Building block models").
    pub step: &'a str,
    /// The line in the bottom-left corner ("Cindral 0.8.0").
    pub version: &'a str,
    /// `1.0` while loading; lower while it fades over the next screen.
    pub opacity: f32,
}

/// The loading screen.
#[derive(Debug, Clone, Copy, Default)]
pub struct LoadingScreen;

impl LoadingScreen {
    /// Creates the screen.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Draws one frame over the whole viewport. `brand` is the brand face
    /// (the game font until it is installed).
    pub fn show(&self, ui: &mut Ui, brand: &FontAtlas, view: &LoadingView<'_>) {
        let viewport = ui.viewport();
        let scale = crate::menus::common::Gui::new(viewport).scale;
        ui.draw().set_opacity(view.opacity);
        ui.draw().rect(viewport, BACKGROUND);

        let cx = viewport.x + viewport.width * 0.5;
        let cy = viewport.y + viewport.height * 0.5;
        let logo = (LOGO * scale).round();
        let logo_top = (cy - logo * 0.5 - GAP * scale * 0.5).round();
        if let Some(sheet) = ui.menu_art().brand.logo {
            let rect = Rect::new((cx - logo * 0.5).round(), logo_top, logo, logo);
            let src = sheet.sub(0, 0, sheet.width, sheet.height);
            ui.draw().image(src, rect, theme::TEXT);
        }

        let bar = bar_rect(viewport, scale, logo_top + logo + GAP * scale);
        ui.draw().rect(bar, TRACK);
        let filled = (bar.width * view.fraction.clamp(0.0, 1.0)).round();
        if filled > 0.0 {
            ui.draw()
                .rect(Rect::new(bar.x, bar.y, filled, bar.height), theme::ACCENT);
        }

        let small = theme::TEXT_SIZE_SMALL * scale;
        let line_y = bar.y + bar.height + 7.0 * scale;
        draw_text(
            ui,
            brand,
            view.step,
            (bar.x, line_y),
            (small, theme::TEXT_MUTED, Align::Left),
        );
        let percent = format!("{}%", (view.fraction.clamp(0.0, 1.0) * 100.0).round());
        draw_text(
            ui,
            brand,
            &percent,
            (bar.x + bar.width, line_y),
            (small, theme::TEXT, Align::Right),
        );

        let corner_y = viewport.y + viewport.height - MARGIN * scale - small * 0.5;
        let faint = theme::with_opacity(theme::TEXT_MUTED, 0.6);
        draw_text(
            ui,
            brand,
            view.version,
            (viewport.x + MARGIN * scale, corner_y),
            (small, faint, Align::Left),
        );
        ui.draw().set_opacity(1.0);
    }
}

/// The bar: [`BAR_WIDTH`] wide (at most 80 % of the window), 2 GUI units
/// high, centred, its top at `top`. Whole pixels, so it never blurs.
fn bar_rect(viewport: Rect, scale: f32, top: f32) -> Rect {
    let width = (BAR_WIDTH * scale).min(viewport.width * 0.8).round();
    let height = (2.0 * scale).round().max(2.0);
    let left = (viewport.x + (viewport.width - width) * 0.5).round();
    Rect::new(left, top.round(), width, height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bar_is_centred_whole_pixels_and_fits_small_windows() {
        let wide = bar_rect(Rect::new(0.0, 0.0, 1280.0, 720.0), 2.0, 400.3);
        assert_eq!(
            (wide.x, wide.y, wide.width, wide.height),
            (490.0, 400.0, 300.0, 4.0)
        );
        let narrow = bar_rect(Rect::new(0.0, 0.0, 200.0, 150.0), 2.0, 10.0);
        assert_eq!(narrow.width, 160.0);
        assert_eq!(narrow.x, 20.0);
        let tiny_scale = bar_rect(Rect::new(0.0, 0.0, 640.0, 480.0), 0.5, 0.0);
        assert_eq!(tiny_scale.height, 2.0, "never thinner than two pixels");
    }
}
