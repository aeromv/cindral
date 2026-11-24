// SPDX-License-Identifier: MIT OR Apache-2.0

//! The card in the top-right corner of the title screen that offers a new
//! version of Cindral, then shows its download and install.

use crate::context::Ui;
use crate::geom::Rect;
use crate::modern::{draw_text, panel, theme, Align, ModernButton};
use crate::text::FontAtlas;

/// Size of the card, in GUI units.
const CARD: (f32, f32) = (148.0, 40.0);
/// Margin from the corner of the screen, in GUI units.
const MARGIN: f32 = 6.0;
/// Inner padding, in GUI units.
const PADDING: f32 = 7.0;
/// Size of the button, in GUI units.
const BUTTON: (f32, f32) = (44.0, 16.0);

/// What the card shows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UpdateView<'a> {
    /// A newer version can be installed.
    Available {
        /// Its version, `0.9.0`.
        version: &'a str,
    },
    /// The download runs; the share done, `0.0..=1.0`.
    Downloading(f32),
    /// The new version is starting.
    Restarting,
    /// The update failed; the button tries again.
    Failed,
}

/// The update card.
#[derive(Debug, Clone, Copy, Default)]
pub struct UpdateCard;

impl UpdateCard {
    /// Draws the card; true when its button was clicked.
    pub fn show(ui: &mut Ui, brand: &FontAtlas, view: UpdateView<'_>) -> bool {
        let viewport = ui.viewport();
        let scale = crate::menus::common::Gui::new(viewport).scale;
        let card = Rect::new(
            viewport.x + viewport.width - (CARD.0 + MARGIN) * scale,
            viewport.y + MARGIN * scale,
            CARD.0 * scale,
            CARD.1 * scale,
        );
        panel(ui, card, scale);
        let left = card.x + PADDING * scale;
        let title_y = card.y + 13.0 * scale;
        let line_y = card.y + 27.0 * scale;
        let body = theme::TEXT_SIZE_BODY * scale;
        let small = theme::TEXT_SIZE_SMALL * scale;
        let (title, line) = match view {
            UpdateView::Available { version } => (format!("Cindral {version} is out"), None),
            UpdateView::Downloading(_) => ("Downloading the update".to_string(), None),
            UpdateView::Restarting => (
                "Installing the update".to_string(),
                Some("Cindral restarts by itself"),
            ),
            UpdateView::Failed => ("The update failed".to_string(), Some("Check your internet")),
        };
        draw_text(
            ui,
            brand,
            &title,
            (left, title_y),
            (body, theme::TEXT, Align::Left),
        );
        if let Some(line) = line {
            draw_text(
                ui,
                brand,
                line,
                (left, line_y),
                (small, theme::TEXT_MUTED, Align::Left),
            );
        }

        let button = Rect::new(
            card.x + card.width - (BUTTON.0 + PADDING) * scale,
            line_y - BUTTON.1 * scale * 0.5,
            BUTTON.0 * scale,
            BUTTON.1 * scale,
        );
        match view {
            UpdateView::Available { .. } | UpdateView::Failed => {
                let label = if view == UpdateView::Failed {
                    "Retry"
                } else {
                    "Update"
                };
                if matches!(view, UpdateView::Available { .. }) {
                    draw_text(
                        ui,
                        brand,
                        "Installs and restarts",
                        (left, line_y),
                        (small, theme::TEXT_MUTED, Align::Left),
                    );
                }
                let id = ui.id("update_card_button");
                ModernButton::new(id, button, label)
                    .primary()
                    .show(ui, brand, scale, 0.0)
                    .clicked
            }
            UpdateView::Downloading(done) => {
                let done = done.clamp(0.0, 1.0);
                let bar = Rect::new(
                    left,
                    line_y - scale,
                    card.width - 2.0 * PADDING * scale - 22.0 * scale,
                    (2.0 * scale).round().max(2.0),
                );
                ui.draw().rect(bar, [1.0, 1.0, 1.0, 0.09]);
                ui.draw().rect(
                    Rect::new(bar.x, bar.y, (bar.width * done).round(), bar.height),
                    theme::ACCENT,
                );
                let percent = format!("{}%", (done * 100.0).round());
                draw_text(
                    ui,
                    brand,
                    &percent,
                    (card.x + card.width - PADDING * scale, line_y),
                    (small, theme::TEXT, Align::Right),
                );
                false
            }
            UpdateView::Restarting => false,
        }
    }
}
