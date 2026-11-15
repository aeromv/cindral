// SPDX-License-Identifier: MIT OR Apache-2.0

//! The layout of the modern title screen, in GUI units times the GUI scale.
//!
//! Everything sits in one column on the left third of the screen, over a
//! shade that darkens that side of the panorama: the logo, the "CINDRAL"
//! wordmark and a tagline, then Play, Options and Quit. The column is centred
//! vertically and keeps a fixed margin from the left edge, so the panorama
//! stays open on the right from the smallest window to a wide monitor.

use crate::geom::Rect;
use crate::menus::common::Gui;

/// Distance of the column from the left edge, at the smallest GUI width.
const MARGIN: f32 = 26.0;
/// Share of the GUI width the margin grows to on wide screens.
const MARGIN_SHARE: f32 = 0.07;
/// Edge of the square logo box.
const LOGO_SIZE: f32 = 30.0;
/// Gap between the logo and the wordmark.
const LOGO_GAP: f32 = 7.0;
/// Height of the wordmark band.
pub(super) const WORDMARK_HEIGHT: f32 = 20.0;
/// Height of the tagline under the wordmark.
const TAGLINE_HEIGHT: f32 = 8.0;
/// Gap between the tagline and the first button.
const TAGLINE_GAP: f32 = 16.0;
/// Size of one button of the column.
const BUTTON_WIDTH: f32 = 150.0;
/// Height of one button of the column.
const BUTTON_HEIGHT: f32 = 21.0;
/// Vertical gap between the buttons.
const BUTTON_GAP: f32 = 5.0;
/// Width of the shade behind the column, as a share of the screen width.
pub(super) const SHADE_SHARE: f32 = 0.55;

/// The rectangles of the modern title screen, in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModernTitleLayout {
    /// The square box the logo is fitted into.
    pub logo: Rect,
    /// The band the "CINDRAL" wordmark starts in, at its left edge.
    pub wordmark: Rect,
    /// The band of the tagline under the wordmark.
    pub tagline: Rect,
    /// The Play button (opens the server list).
    pub play: Rect,
    /// The Options button.
    pub options: Rect,
    /// The Quit button.
    pub quit: Rect,
}

/// Computes the modern layout for `viewport`.
///
/// The column is centred vertically as one block, so no part leaves the
/// window at the smallest GUI height (240).
#[must_use]
pub fn modern_title_layout(viewport: Rect) -> ModernTitleLayout {
    let gui = Gui::new(viewport);
    let left = MARGIN.max((gui.width * MARGIN_SHARE).floor());
    let block = LOGO_SIZE
        + LOGO_GAP
        + WORDMARK_HEIGHT
        + TAGLINE_HEIGHT
        + TAGLINE_GAP
        + 3.0 * BUTTON_HEIGHT
        + 2.0 * BUTTON_GAP;
    let top = ((gui.height - block) * 0.5).floor();
    let wordmark_top = top + LOGO_SIZE + LOGO_GAP;
    let tagline_top = wordmark_top + WORDMARK_HEIGHT;
    let first = tagline_top + TAGLINE_HEIGHT + TAGLINE_GAP;
    let button = |row: f32| {
        let y = first + row * (BUTTON_HEIGHT + BUTTON_GAP);
        gui.rect(left, y, BUTTON_WIDTH, BUTTON_HEIGHT)
    };
    ModernTitleLayout {
        logo: gui.rect(left, top, LOGO_SIZE, LOGO_SIZE),
        wordmark: gui.rect(left, wordmark_top, BUTTON_WIDTH, WORDMARK_HEIGHT),
        tagline: gui.rect(left, tagline_top, BUTTON_WIDTH, TAGLINE_HEIGHT),
        play: button(0.0),
        options: button(1.0),
        quit: button(2.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menus::common::testing::VIEWPORT;

    #[test]
    fn the_column_is_left_aligned() {
        let layout = modern_title_layout(VIEWPORT);
        for rect in [
            layout.logo,
            layout.wordmark,
            layout.tagline,
            layout.play,
            layout.options,
            layout.quit,
        ] {
            assert!((rect.x - layout.play.x).abs() < 1e-3, "{rect:?}");
        }
        assert!(layout.play.x > VIEWPORT.x);
        assert!(layout.play.x + layout.play.width < VIEWPORT.x + VIEWPORT.width * 0.5);
    }

    #[test]
    fn the_parts_stack_top_to_bottom_without_overlap() {
        let layout = modern_title_layout(VIEWPORT);
        let bottom = |rect: Rect| rect.y + rect.height;
        assert!(bottom(layout.logo) <= layout.wordmark.y);
        assert!(bottom(layout.wordmark) <= layout.tagline.y);
        assert!(bottom(layout.tagline) <= layout.play.y);
        assert!(bottom(layout.play) <= layout.options.y);
        assert!(bottom(layout.options) <= layout.quit.y);
        assert!(bottom(layout.quit) <= VIEWPORT.y + VIEWPORT.height);
    }

    #[test]
    fn the_column_is_centred_vertically() {
        let layout = modern_title_layout(VIEWPORT);
        let above = layout.logo.y - VIEWPORT.y;
        let below = VIEWPORT.y + VIEWPORT.height - (layout.quit.y + layout.quit.height);
        let unit = Gui::new(VIEWPORT).scale;
        assert!((above - below).abs() <= 2.0 * unit, "{above} vs {below}");
    }

    #[test]
    fn the_smallest_window_keeps_everything_visible() {
        let viewport = Rect::new(0.0, 0.0, 320.0, 240.0);
        let layout = modern_title_layout(viewport);
        assert!(layout.logo.y >= 0.0);
        assert!(layout.quit.y + layout.quit.height <= 240.0);
        assert!(layout.quit.x + layout.quit.width <= 320.0);
    }
}
