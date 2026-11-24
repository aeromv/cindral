// SPDX-License-Identifier: MIT OR Apache-2.0

//! What the options do outside their screen (ADR-0043): the settings the
//! server is told, the video options applied to the window and renderer, and
//! the resource pack list the loaders get.
//!
//! The window calls these from its frame and from the moment the options
//! close; each piece is kept small and pure where it can be, so it is tested
//! without a GPU or a window.

use std::path::{Path, PathBuf};

use cindral_assets::cache::data_dir;
use cindral_render::Renderer;
use winit::window::{Fullscreen, Window};

use crate::game::GameEvent;
use crate::options::video_sections::LeavesMode;
use crate::options::video_spec::{effect_of_change, Effect};
use crate::options::{Options, VideoOptions, PACKS_FOLDER};
use crate::scene::entity::EntityTextures;

/// The options of `Client Settings` (0x15): what the server learns about the
/// player. Compared before and after the options screen to decide whether to
/// send the packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerSettings {
    /// The render distance in chunks.
    pub view_distance: u8,
    /// The chat visibility byte.
    pub chat_mode: u8,
    /// Whether the chat shows colours.
    pub chat_colors: bool,
    /// The skin-parts byte (`SkinOptions::parts_mask`).
    pub skin_parts: u8,
}

impl ServerSettings {
    /// The settings `options` select.
    #[must_use]
    pub fn of(options: &Options) -> Self {
        Self {
            view_distance: options.video.render_distance,
            chat_mode: options.chat.visibility.wire(),
            chat_colors: options.chat.colors,
            skin_parts: options.skin.parts_mask(),
        }
    }

    /// The event that sends these settings from the game thread.
    #[must_use]
    pub fn event(self) -> GameEvent {
        GameEvent::ClientSettings {
            view_distance: self.view_distance,
            chat_mode: self.chat_mode,
            chat_colors: self.chat_colors,
            skin_parts: self.skin_parts,
        }
    }
}

/// The folder the resource packs live in, when the platform has a data
/// directory.
#[must_use]
pub fn packs_folder() -> Option<PathBuf> {
    data_dir().map(|folder| folder.join(PACKS_FOLDER))
}

/// The packs to load, lowest priority first.
///
/// A `--resource-pack` on the command line wins over the saved list, as the
/// command-line render distance does over the saved one. The pack of the
/// connected server comes last, so it wins over both (the reference client
/// puts it on top of the repository's pack list).
#[must_use]
pub fn pack_paths(
    cli: &[PathBuf],
    options: &Options,
    folder: Option<&Path>,
    server_pack: Option<&Path>,
) -> Vec<PathBuf> {
    let mut paths = if cli.is_empty() {
        folder.map_or_else(Vec::new, |folder| options.resource_packs.paths_in(folder))
    } else {
        cli.to_vec()
    };
    paths.extend(server_pack.map(Path::to_path_buf));
    paths
}

/// Opens `folder` in the file manager of the platform (the "Open resource
/// pack folder" button). A failure is logged; the screen goes on.
pub fn open_folder(folder: &Path) {
    if let Err(error) = std::fs::create_dir_all(folder) {
        tracing::warn!(folder = %folder.display(), %error, "could not create the packs folder");
        return;
    }
    let opener = if cfg!(target_os = "windows") {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    if let Err(error) = std::process::Command::new(opener).arg(folder).spawn() {
        tracing::warn!(opener, %error, "could not open the packs folder");
    }
}

/// Applies the video options that act on the window, the renderer and the
/// GUI, once per frame, and reports what a change asks the window to redo.
/// It remembers the options it last applied, so a steady frame only compares
/// them.
#[derive(Debug, Default)]
pub struct VideoApplier {
    fullscreen: bool,
    /// The Display resolution the fullscreen window was last set with.
    display: String,
    /// The game is in the background (Alt+Tab): its fullscreen resolution
    /// gives way to the desktop one when the option asks for it.
    suspended: bool,
    /// Where the window was and how big, the last time it was not
    /// fullscreen: leaving fullscreen puts it back there.
    windowed: Option<(
        winit::dpi::PhysicalPosition<i32>,
        winit::dpi::PhysicalSize<u32>,
    )>,
    /// The options of the previous frame, once there was one.
    applied: Option<VideoOptions>,
}

impl VideoApplier {
    /// Creates a new `VideoApplier` initialized with `fullscreen`.
    #[must_use]
    pub fn new(fullscreen: bool) -> Self {
        Self {
            fullscreen,
            display: String::new(),
            suspended: false,
            windowed: None,
            applied: None,
        }
    }

    /// Tells the applier whether the game window has the focus. Losing it in
    /// a fullscreen game at another resolution, with the option on, gives
    /// the desktop its resolution back at once (before the window goes to
    /// the taskbar); the next frame with the focus switches back.
    pub fn set_focused(&mut self, focused: bool, video: &VideoOptions, window: Option<&Window>) {
        self.suspended = !focused;
        if focused || !video.alt_tab_desktop || !self.fullscreen || self.display.is_empty() {
            return;
        }
        if let Some(window) = window {
            window.set_fullscreen(Some(Fullscreen::Borderless(None)));
            self.display.clear();
        }
    }

    fn remember_windowed(&mut self, window: &Window) {
        if let Ok(position) = window.outer_position() {
            self.windowed = Some((position, window.inner_size()));
        }
    }

    /// Puts a window that left fullscreen back where it was; one that never
    /// was a window (the game started fullscreen) is centred on its monitor
    /// at 1280x720, or less on a small screen.
    fn restore_windowed(&self, window: &Window) {
        if let Some((position, size)) = self.windowed {
            let _ = window.request_inner_size(size);
            window.set_outer_position(position);
            return;
        }
        let Some(monitor) = window.current_monitor() else {
            return;
        };
        let screen = monitor.size();
        let size = winit::dpi::PhysicalSize::new(
            1280.min(screen.width * 3 / 4),
            720.min(screen.height * 3 / 4),
        );
        let origin = monitor.position();
        let _ = window.request_inner_size(size);
        window.set_outer_position(winit::dpi::PhysicalPosition::new(
            origin.x + i32::try_from((screen.width - size.width) / 2).unwrap_or(0),
            origin.y + i32::try_from((screen.height - size.height) / 2).unwrap_or(0),
        ));
    }

    /// The fullscreen state to switch to, or `None` when `wanted` is what is
    /// already set.
    fn fullscreen_change(&mut self, wanted: bool, display: &str) -> Option<bool> {
        let display_changed = wanted && self.display != display;
        (self.fullscreen != wanted || display_changed).then(|| {
            self.fullscreen = wanted;
            self.display = display.to_owned();
            wanted
        })
    }

    /// The effect of the options that changed since the previous frame
    /// (ADR-0089). The first call only records `video`: the resources were
    /// loaded from the same options at start-up.
    fn change(&mut self, video: &VideoOptions) -> Effect {
        if self.applied.as_ref() == Some(video) {
            return Effect::Immediate;
        }
        let effect = self
            .applied
            .as_ref()
            .map_or(Effect::Immediate, |before| effect_of_change(before, video));
        self.applied = Some(video.clone());
        effect
    }

    /// Applies `video` for this frame: the GUI scale, vsync, fullscreen, the
    /// entity shadows, the mip levels, cave culling, the opaque leaves of
    /// "Trees: Fast" (`leaf_layers`) and the mesh options.
    /// Brightness, bobbing and the cloud mode are read where they are used
    /// (the render environment, the camera and the cloud pass). Returns what
    /// the window has to redo.
    #[must_use]
    pub fn apply(
        &mut self,
        video: &VideoOptions,
        window: Option<&Window>,
        renderer: &mut Renderer,
        entities: Option<&EntityTextures>,
        leaf_layers: &[u32],
    ) -> Effect {
        cindral_ui::set_gui_scale_option(u32::from(video.gui_scale));
        renderer.set_vsync(video.vsync);
        renderer.set_cave_culling(video.cave_culling);
        renderer.set_render_scale(f32::from(video.render_scale) / 100.0, true);
        if let Some(entities) = entities {
            entities.set_shadows_enabled(video.entity_shadows);
            entities.set_custom_models_enabled(video.quality.custom_entity_models);
        }
        let display = if self.suspended && video.alt_tab_desktop {
            ""
        } else {
            video.display.as_str()
        };
        if let Some(window) = window {
            if !self.fullscreen
                && window.fullscreen().is_none()
                && !window.is_minimized().unwrap_or(false)
            {
                self.remember_windowed(window);
            }
            if let Some(fullscreen) = self.fullscreen_change(video.fullscreen, display) {
                if fullscreen {
                    window.set_fullscreen(Some(fullscreen_mode(window, display)));
                } else {
                    // Leaving fullscreen always gives the desktop its own
                    // resolution back, then the window its old place.
                    window.set_fullscreen(None);
                    self.restore_windowed(window);
                }
            }
        }
        renderer.set_mipmap_levels(u32::from(video.mipmap_levels));
        let fast_leaves = video.details.trees.resolve(video.fancy_graphics) == LeavesMode::Fast;
        renderer.set_opaque_layers(if fast_leaves { leaf_layers } else { &[] });
        crate::meshing::settings::store(video);
        self.change(video)
    }
}

/// The fullscreen of `display`: the monitor switched to that resolution
/// (its highest refresh rate) when it offers it, else borderless at the
/// desktop resolution, which needs no mode switch.
fn fullscreen_mode(window: &Window, display: &str) -> Fullscreen {
    let Some((width, height)) = super::video::display_size(display) else {
        return Fullscreen::Borderless(None);
    };
    let mode = window.current_monitor().and_then(|monitor| {
        monitor
            .video_modes()
            .filter(|mode| {
                let size = mode.size();
                (size.width, size.height) == (width, height)
            })
            .max_by_key(|mode| (mode.bit_depth(), mode.refresh_rate_millihertz()))
    });
    match mode {
        Some(mode) => {
            tracing::info!(width, height, "fullscreen at the Display resolution");
            Fullscreen::Exclusive(mode)
        }
        None => {
            tracing::warn!(
                width,
                height,
                "the monitor has no such mode; borderless instead"
            );
            Fullscreen::Borderless(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::SkinPart;

    #[test]
    fn the_server_settings_follow_the_options() {
        let mut options = Options::default();
        options.video.render_distance = 12;
        options.chat.colors = false;
        options.skin.toggle(SkinPart::Hat);
        let settings = ServerSettings::of(&options);
        assert_eq!(settings.view_distance, 12);
        assert!(!settings.chat_colors);
        assert_eq!(settings.skin_parts, 0b0011_1111);
    }

    #[test]
    fn a_skin_change_alone_changes_the_server_settings() {
        let before = Options::default();
        let mut after = before.clone();
        assert_eq!(ServerSettings::of(&before), ServerSettings::of(&after));
        after.skin.toggle(SkinPart::Cape);
        assert_ne!(ServerSettings::of(&before), ServerSettings::of(&after));
    }

    #[test]
    fn the_event_carries_every_field() {
        let settings = ServerSettings {
            view_distance: 6,
            chat_mode: 2,
            chat_colors: false,
            skin_parts: 5,
        };
        let GameEvent::ClientSettings {
            view_distance,
            chat_mode,
            chat_colors,
            skin_parts,
        } = settings.event()
        else {
            panic!("not a ClientSettings event");
        };
        assert_eq!(
            (view_distance, chat_mode, chat_colors, skin_parts),
            (6, 2, false, 5)
        );
    }

    #[test]
    fn the_command_line_packs_win_over_the_saved_list() {
        let mut options = Options::default();
        options.resource_packs.enabled = vec!["saved.zip".to_string()];
        let folder = Path::new("/data/resourcepacks");
        let cli = vec![PathBuf::from("cli_pack")];
        assert_eq!(pack_paths(&cli, &options, Some(folder), None), cli);
        assert_eq!(
            pack_paths(&[], &options, Some(folder), None),
            [folder.join("saved.zip")]
        );
        assert!(pack_paths(&[], &options, None, None).is_empty());
    }

    #[test]
    fn the_server_pack_has_the_top_priority() {
        let mut options = Options::default();
        options.resource_packs.enabled = vec!["saved.zip".to_string()];
        let folder = Path::new("/data/resourcepacks");
        let server = Path::new("/cache/server-resourcepacks/abc");
        assert_eq!(
            pack_paths(&[], &options, Some(folder), Some(server)),
            [folder.join("saved.zip"), server.to_path_buf()]
        );
        let cli = vec![PathBuf::from("cli_pack")];
        assert_eq!(
            pack_paths(&cli, &options, Some(folder), Some(server)),
            [PathBuf::from("cli_pack"), server.to_path_buf()]
        );
        assert_eq!(
            pack_paths(&[], &options, None, Some(server)),
            [server.to_path_buf()]
        );
    }

    #[test]
    fn a_change_reports_its_effect_once() {
        let mut applier = VideoApplier::default();
        let mut video = VideoOptions::default();
        assert_eq!(
            applier.change(&video),
            Effect::Immediate,
            "the first frame only records"
        );
        assert_eq!(applier.change(&video), Effect::Immediate);
        video.mipmap_levels = 2;
        assert_eq!(applier.change(&video), Effect::ReloadResources);
        assert_eq!(applier.change(&video), Effect::Immediate);
        video.smooth_lighting = 0;
        assert_eq!(applier.change(&video), Effect::ReMesh);
        video.view_bobbing = false;
        assert_eq!(applier.change(&video), Effect::Immediate);
    }

    #[test]
    fn fullscreen_is_only_switched_when_it_changes() {
        let mut applier = VideoApplier::default();
        assert_eq!(applier.fullscreen_change(false, ""), None);
        assert_eq!(applier.fullscreen_change(true, ""), Some(true));
        assert_eq!(applier.fullscreen_change(true, ""), None);
        assert_eq!(applier.fullscreen_change(false, ""), Some(false));
    }

    #[test]
    fn a_new_display_resolution_switches_only_in_fullscreen() {
        let mut applier = VideoApplier::default();
        assert_eq!(
            applier.fullscreen_change(false, "1600x900"),
            None,
            "a window ignores the Display option"
        );
        assert_eq!(applier.fullscreen_change(true, "1600x900"), Some(true));
        assert_eq!(applier.fullscreen_change(true, "1600x900"), None);
        assert_eq!(applier.fullscreen_change(true, "1280x720"), Some(true));
        assert_eq!(super::super::display_size("1280x720"), Some((1280, 720)));
        assert_eq!(super::super::display_size(""), None);
    }
}
