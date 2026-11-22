// SPDX-License-Identifier: MIT OR Apache-2.0

//! The splash screen shown while the textures, the packs and the sounds load
//! in the background: the Cindral logo, the wordmark, its author, and a bar
//! that keeps moving so the start never looks frozen.

use std::sync::OnceLock;
use std::time::Instant;

use cindral_core::lang::Lang;

use crate::context::Ui;
use crate::geom::Rect;
use crate::modern::{draw_text, measure_text, theme, Align};
use crate::text::FontAtlas;

/// Height of the wordmark letters, in GUI units.
const WORD_SIZE: f32 = 16.0;
/// Extra space after each letter of the wordmark, as a fraction of its size.
const TRACKING: f32 = 0.3;
/// Edge of the logo, in GUI units.
const LOGO: f32 = 34.0;
/// Size of the bar, in GUI units.
const BAR: (f32, f32) = (110.0, 2.0);
/// Length of the moving part of the bar, as a fraction of the bar.
const RUNNER: f32 = 0.32;
/// Seconds for the moving part to cross the bar.
const PERIOD: f32 = 1.1;

/// The loading splash screen.
#[derive(Debug, Clone, Copy, Default)]
pub struct LoadingScreen;

impl LoadingScreen {
    /// Creates the screen.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Draws one frame. `brand` is the brand face (the game font until it
    /// is installed); the bar moves with the time since the first frame.
    pub fn show(&self, ui: &mut Ui, brand: &FontAtlas, lang: &Lang) {
        static START: OnceLock<Instant> = OnceLock::new();
        let seconds = START.get_or_init(Instant::now).elapsed().as_secs_f32();
        let viewport = ui.viewport();
        let scale = crate::menus::common::Gui::new(viewport).scale;
        ui.draw()
            .vertical_gradient(viewport, theme::BACKDROP_TOP, theme::BACKDROP_BOTTOM);

        let cx = viewport.x + viewport.width * 0.5;
        let cy = viewport.y + viewport.height * 0.5;
        // A slow pulse on the logo, between 75 % and 100 %.
        let pulse = 0.875 + 0.125 * (seconds * 2.4).sin();
        let logo_edge = LOGO * scale;
        let logo_top = cy - 46.0 * scale;
        if let Some(sheet) = ui.menu_art().brand.logo {
            let rect = Rect::new(cx - logo_edge * 0.5, logo_top, logo_edge, logo_edge);
            let src = sheet.sub(0, 0, sheet.width, sheet.height);
            ui.draw()
                .image(src, rect, theme::with_opacity(theme::ACCENT, pulse));
        }

        let word_y = logo_top + logo_edge + 14.0 * scale;
        draw_tracked(ui, brand, "CINDRAL", (cx, word_y), WORD_SIZE * scale);
        draw_text(
            ui,
            brand,
            "by AeroMV",
            (cx, word_y + 13.0 * scale),
            (
                theme::TEXT_SIZE_SMALL * scale,
                theme::TEXT_MUTED,
                Align::Center,
            ),
        );

        let bar_y = word_y + 26.0 * scale;
        let bar = Rect::new(
            cx - BAR.0 * scale * 0.5,
            bar_y,
            BAR.0 * scale,
            (BAR.1 * scale).max(2.0),
        );
        ui.draw()
            .rounded_rect(bar, bar.height * 0.5, [1.0, 1.0, 1.0, 0.08]);
        let (start, end) = runner(seconds);
        if end > start {
            let moving = Rect::new(
                bar.x + bar.width * start,
                bar.y,
                bar.width * (end - start),
                bar.height,
            );
            ui.draw()
                .rounded_rect(moving, bar.height * 0.5, theme::ACCENT);
        }

        let message = lang.get("menu.loading").unwrap_or("Loading resources...");
        draw_text(
            ui,
            brand,
            message,
            (cx, bar_y + 12.0 * scale),
            (
                theme::TEXT_SIZE_SMALL * scale,
                theme::with_opacity(theme::TEXT_MUTED, 0.8),
                Align::Center,
            ),
        );
    }
}

/// The lit part of the bar at `seconds`, as `(start, end)` fractions: a
/// runner that enters on the left and leaves on the right.
fn runner(seconds: f32) -> (f32, f32) {
    let phase = (seconds / PERIOD).fract();
    let head = phase * (1.0 + RUNNER);
    ((head - RUNNER).max(0.0), head.min(1.0))
}

/// Draws `text` centred on `centre` with wide letter spacing.
fn draw_tracked(ui: &mut Ui, brand: &FontAtlas, text: &str, centre: (f32, f32), size: f32) {
    let gap = size * TRACKING;
    let mut buffer = [0u8; 4];
    let widths: Vec<f32> = text
        .chars()
        .map(|letter| measure_text(brand, letter.encode_utf8(&mut buffer), size))
        .collect();
    let total = widths.iter().sum::<f32>() + gap * (widths.len().saturating_sub(1)) as f32;
    let mut x = centre.0 - total * 0.5;
    for (letter, width) in text.chars().zip(widths) {
        draw_text(
            ui,
            brand,
            letter.encode_utf8(&mut buffer),
            (x, centre.1),
            (size, theme::TEXT, Align::Left),
        );
        x += width + gap;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_runner_crosses_the_bar_and_stays_inside_it() {
        assert_eq!(runner(0.0), (0.0, 0.0));
        for step in 0..100 {
            let (start, end) = runner(step as f32 * 0.037);
            assert!((0.0..=1.0).contains(&start) && (0.0..=1.0).contains(&end));
            assert!(start <= end);
        }
        let (start, end) = runner(PERIOD * 0.5);
        assert!(end - start > 0.2, "half way the runner is fully in");
    }
}
