// SPDX-License-Identifier: MIT OR Apache-2.0

//! The Display page: the screen resolution of the fullscreen game, chosen in
//! a list of every resolution the monitor offers, and whether Alt+Tab gives
//! the desktop its own resolution back. Not vanilla: OptiFine has the same
//! list as "Fullscreen Mode".

use cindral_core::lang::Lang;
use cindral_ui::{FontAtlas, Ui};

use super::layout::{Gui, BUTTON_H, GREY, HALF_W};
use super::{option_label, OptionsScreen, Outcome};
use crate::options::{display_modes, Options};

/// Height of one row of the list.
const ROW: f32 = 18.0;
/// Width of the selection box.
const LIST_WIDTH: f32 = 220.0;
/// Top of the list, under the title and the Alt+Tab switch.
const LIST_TOP: f32 = 58.0;

impl OptionsScreen {
    /// Draws the page: the Alt+Tab switch, the list, the note and Done.
    pub(super) fn show_display(
        &mut self,
        ui: &mut Ui,
        atlas: &FontAtlas,
        options: &mut Options,
        gui: Gui,
        lang: &Lang,
        outcome: &mut Outcome,
    ) {
        let cx = gui.cx();
        // "Desktop" first, then the monitor's resolutions, largest first.
        let mut rows = vec![(
            String::new(),
            lang.tr("cindral.options.display.desktop").to_string(),
        )];
        rows.extend(display_modes::get().into_iter().map(|size| {
            (
                display_modes::format(size),
                format!("{} x {}", size.0, size.1),
            )
        }));
        let (top, bottom) = (LIST_TOP, gui.height - 61.0);
        let content = rows.len() as f32 * ROW;
        self.scroll_list(ui, gui, (cx + 124.0, top, bottom), (content, ROW));
        Self::list_container(ui, gui, (0.0, gui.width), (top, bottom), self.scroll);
        ui.push_clip(gui.rect(0.0, top, gui.width, bottom - top));
        let mut picked = None;
        for (index, (value, label)) in rows.iter().enumerate() {
            let y = top + 4.0 - self.scroll + index as f32 * ROW;
            let rect = gui.rect(cx - LIST_WIDTH / 2.0, y - 2.0, LIST_WIDTH, ROW);
            if *value == options.video.display {
                Self::selection_box(ui, gui, cx, y);
            }
            Self::centered_text(ui, atlas, gui, (cx, y + 1.0), label, [1.0; 4]);
            let id = format!("display:{value}");
            self.rects.insert(id.clone(), rect);
            let widget = ui.id(&id);
            if ui.response(widget, rect).clicked {
                picked = Some(value.clone());
            }
        }
        ui.pop_clip();
        Self::list_overlay(ui, gui, (0.0, gui.width), (top, bottom));
        Self::scrollbar(ui, gui, cx + 124.0, (top, bottom), content, self.scroll);
        if let Some(value) = picked.filter(|value| *value != options.video.display) {
            options.video.display = value;
            outcome.changed = true;
        }

        Self::title(ui, atlas, gui, lang.tr("cindral.options.display"), 12.0);
        // After the list: its top band would cover the switch.
        let alt_tab = gui.rect(cx - 155.0, 28.0, 310.0, BUTTON_H);
        let state = lang.tr(if options.video.alt_tab_desktop {
            "options.on"
        } else {
            "options.off"
        });
        let label = option_label(lang, "cindral.options.display.altTab", state);
        if self.button(ui, atlas, "display:alt_tab", alt_tab, &label, true, None) {
            options.video.alt_tab_desktop = !options.video.alt_tab_desktop;
            outcome.changed = true;
        }

        let note = lang.tr("cindral.options.display.note");
        Self::centered_text(ui, atlas, gui, (cx, gui.height - 56.0), note, GREY);
        let done = gui.rect(cx - 155.0 + 80.0, gui.height - 38.0, HALF_W, BUTTON_H);
        self.back(ui, atlas, done, lang);
    }
}
