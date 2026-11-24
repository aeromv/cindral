// SPDX-License-Identifier: MIT OR Apache-2.0

//! The hub page (`GuiOptions`).

use cindral_core::lang::Lang;
use cindral_ui::menus::TitleStyle;
use cindral_ui::{FontAtlas, Ui};

use super::layout::{Gui, BUTTON_H, HALF_W};
use super::{option_label, OptionsScreen, Outcome, Page};
use crate::options::video_spec::VideoPage;
use crate::options::{Options, FOV};

impl OptionsScreen {
    /// `GuiOptions`.
    pub(super) fn show_main(
        &mut self,
        ui: &mut Ui,
        atlas: &FontAtlas,
        options: &mut Options,
        gui: Gui,
        lang: &Lang,
        outcome: &mut Outcome,
    ) {
        Self::title(ui, atlas, gui, lang.tr("options.title"), 15.0);
        let cx = gui.cx();
        let h6 = (gui.height / 6.0).floor();
        let left = cx - 155.0;
        let right = cx + 5.0;

        let fov = options.video.fov;
        let fraction = (fov - FOV.start()) / (FOV.end() - FOV.start());
        let rect = gui.rect(left, h6 - 12.0, HALF_W, BUTTON_H);
        if let Some(value) = self.slider(ui, atlas, "fov", rect, &fov_label(lang, fov), fraction) {
            options.video.fov = (FOV.start() + value * (FOV.end() - FOV.start())).round();
            outcome.changed = true;
        }
        // The difficulty belongs to the server (`isSingleplayer()` is false).
        let difficulty = gui.rect(right, h6 - 12.0, HALF_W, BUTTON_H);
        self.button(
            ui,
            atlas,
            "difficulty",
            difficulty,
            &option_label(
                lang,
                "options.difficulty",
                lang.tr("options.difficulty.normal"),
            ),
            false,
            None,
        );

        // Cindral's own option, in the free row between the sliders and the pages.
        let style = gui.rect(left, h6 + 18.0, HALF_W, BUTTON_H);
        let label = option_label(
            lang,
            "cindral.options.titleStyle",
            title_style_text(lang, options),
        );
        if self.button(ui, atlas, "title_style", style, &label, true, None) {
            options.menu.toggle_title_style();
            outcome.changed = true;
        }

        let theme = gui.rect(right, h6 + 18.0, HALF_W, BUTTON_H);
        if self.button(
            ui,
            atlas,
            "theme",
            theme,
            lang.tr("cindral.options.theme"),
            true,
            None,
        ) {
            self.open(Page::Theme, options);
        }

        // (id, label key, y offset, x). Super Secret Settings and the Twitch
        // stream are gone: the client has nothing behind them. The Display
        // page (fullscreen resolution) closes the list.
        let entries: [(&str, &str, f32, f32); 8] = [
            ("skin", "options.skinCustomisation", 48.0, left),
            ("sounds", "options.sounds", 48.0, right),
            ("nav:video", "options.video", 72.0, left),
            ("nav:controls", "options.controls", 72.0, right),
            ("language", "options.language", 96.0, left),
            ("nav:chat", "options.chat.title", 96.0, right),
            ("resourcepacks", "options.resourcepack", 120.0, left),
            ("snooper", "options.snooper.view", 120.0, right),
        ];
        for (id, key, row, x) in entries {
            let rect = gui.rect(x, h6 + row - 6.0, HALF_W, BUTTON_H);
            if self.button(ui, atlas, id, rect, lang.tr(key), true, None) {
                let page = match id {
                    "nav:video" => Page::Video(VideoPage::Root),
                    "nav:controls" => Page::Controls,
                    "nav:chat" => Page::Chat,
                    "skin" => Page::Skin,
                    "sounds" => Page::Sounds,
                    "language" => Page::Language,
                    "resourcepacks" => Page::ResourcePacks,
                    "snooper" => Page::Snooper,
                    _ => continue,
                };
                self.open(page, options);
            }
        }

        // Display, across the row: the resolution in use, a click for the list.
        let display = gui.rect(left, h6 + 138.0, 310.0, BUTTON_H);
        let label = display_text(lang, options);
        if self.button(ui, atlas, "display", display, &label, true, None) {
            self.open(Page::Display, options);
        }

        let done = gui.rect(cx - 100.0, h6 + 168.0, 200.0, BUTTON_H);
        if self.button(ui, atlas, "done", done, lang.tr("gui.done"), true, None) {
            outcome.done = true;
        }
    }
}

/// The Display resolution as the hub shows it: "Display: 1600 x 900".
fn display_text(lang: &Lang, options: &Options) -> String {
    let value = crate::options::display_size(&options.video.display).map_or_else(
        || lang.tr("cindral.options.display.desktop").to_string(),
        |(width, height)| format!("{width} x {height}"),
    );
    option_label(lang, "cindral.options.display", &value)
}

/// The translated name of the current title screen style.
pub(super) fn title_style_text<'a>(lang: &'a Lang, options: &Options) -> &'a str {
    lang.tr(match options.menu.style() {
        TitleStyle::Modern => "cindral.options.titleStyle.modern",
        TitleStyle::Classic => "cindral.options.titleStyle.classic",
    })
}

/// `GameSettings.getKeyBinding(FOV)`.
pub(super) fn fov_label(lang: &Lang, fov: f32) -> String {
    let value = if fov == 70.0 {
        lang.tr("options.fov.min").to_string()
    } else if fov == 110.0 {
        lang.tr("options.fov.max").to_string()
    } else {
        (fov as i32).to_string()
    };
    option_label(lang, "options.fov", &value)
}
