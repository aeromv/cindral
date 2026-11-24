// SPDX-License-Identifier: MIT OR Apache-2.0

//! Starting the window: the state of the app, the assets loaded in the
//! background and the textures uploaded once they arrive.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use cindral_assets::PackFiles;
use cindral_core::lang::Lang;
use cindral_game::chat::{ChatInput, SentHistory};
use cindral_render::{Camera, HudState, Overlay, Renderer, SkyImage, SkyTextures};
use cindral_ui::game_menu::GameMenu;
use cindral_ui::{debug_screen::DebugScreen, FontAtlas, Rect, Ui};

use super::server_pack_control;
use super::{
    App, Assets, ChatUi, FrameClock, FrameScratch, InputTracking, OptionsMenu, PendingPresses,
};
use crate::accounts::AccountsFlow;
use crate::args::Args;
use crate::audio_bridge::AudioBridge;
use crate::debug::cpu_split::CpuSplit;
use crate::error::ClientError;
use crate::hud::cache::HudCache;
use crate::hud::dials::DialAnimator;
use crate::input::ui::UiInputTranslator;
use crate::input::InputState;
use crate::meshing::pool::worker_count;
use crate::mods::runtime::ModsRuntime;
use crate::mods::zoom::Zoom;
use crate::options::effects::{pack_paths, packs_folder, VideoApplier};
use crate::options::Options;
use crate::resources::custom_animations::{self, CustomAnimation};
use crate::resources::custom_guis::CustomGuis;
use crate::resources::custom_items::CustomItems;
use crate::resources::entity_icons::EntityIcons;
use crate::resources::game_files::{load_packs, GameFiles, PackFeatures};
use crate::resources::load_progress::{LoadProgress, Splash, Step};
use crate::resources::panorama::load_panorama_faces;
use crate::resources::sky::{load_border_texture, load_sky_textures, CustomSky};
use crate::resources::terrain::{InstalledTerrain, TerrainTextures};
use crate::resources::texture_animator::TextureAnimator;
use crate::resources::theme_font::FontChoice;
use crate::resources::ui_font::{self, LoadedFont};
use crate::scene::entity::EntityTextures;
use crate::scene::item_world::{ItemLayers, WorldItems};
use crate::scene::map::MapTextures;
use crate::screens::anvil_field::AnvilField;
use crate::screens::creative::input::CreativeInput;
use crate::screens::inventory::input::InventoryInput;
use crate::screens::menu_flow::MenuFlow;
use crate::screens::options::{OptionsScreen, Outcome as OptionsOutcome};
use crate::screens::text::TextScreens;
use crate::screens::Screen;
use crate::servers::{Pinger, ServerBook};
use crate::world_view::WorldView;

/// Decoded asset data delivered asynchronously by the background asset loader.
pub(super) struct AsyncLoadedAssets {
    pub(super) font: LoadedFont,
    pub(super) terrain: TerrainTextures,
    pub(super) files: GameFiles,
    pub(super) custom_animations: Vec<CustomAnimation>,
    pub(super) pictures: DecodedPictures,
}

/// What is left to upload, one stage per frame.
pub(in crate::window) enum PendingUpload {
    /// Decoded; nothing is on the graphics card yet.
    Decoded(AsyncLoadedAssets),
    /// The font and the block textures are uploaded; the rest follows.
    Terrain {
        terrain: InstalledTerrain,
        rest: AfterTerrain,
    },
}

/// The assets still to upload once the block textures are.
pub(in crate::window) struct AfterTerrain {
    files: GameFiles,
    custom_animations: Vec<CustomAnimation>,
    pictures: DecodedPictures,
}

/// The sky and title pictures, decoded on the asset thread so the window
/// only uploads them. Decoding them on the window thread held the loading
/// screen still for half a second on a Pentium G5400 (a pack's custom sky
/// is several large PNGs).
pub(super) struct DecodedPictures {
    sky: SkyTextures,
    border: Option<SkyImage>,
    custom_sky: CustomSky,
    custom_sky_images: Vec<SkyImage>,
    panorama: Option<[SkyImage; 6]>,
}

impl DecodedPictures {
    /// Decodes every picture from `files` and the verified jar.
    pub(super) fn load(files: &GameFiles) -> Self {
        let (custom_sky, custom_sky_images) = CustomSky::load(files);
        let jar = crate::resources::official::find_verified_jar().ok();
        Self {
            sky: load_sky_textures(files),
            border: load_border_texture(files),
            custom_sky,
            custom_sky_images,
            panorama: load_panorama_faces(jar.as_ref()),
        }
    }
}

/// Reads the packs at `paths` once and builds from them both the terrain
/// (atlases and models) and the [`GameFiles`] the other textures are read
/// from, over the verified jar.
pub(super) fn load_packs_and_terrain(
    paths: &[PathBuf],
    features: PackFeatures,
    progress: &LoadProgress,
) -> (TerrainTextures, GameFiles, Vec<CustomAnimation>) {
    progress.set(Step::Packs);
    let mut packs = load_packs(paths);
    progress.set(Step::Models);
    let pack_files = PackFiles::from_pack_refs(&packs);
    // The pictures of the custom container screens ride in a synthetic pack on
    // top, so the GUI atlas holds them as regions.
    let (guis, guis_pack) = CustomGuis::scan(&pack_files, features);
    packs.extend(guis_pack);
    // The same for the pictures and generated models of the custom items.
    let (items, items_pack) = CustomItems::scan(&pack_files, features);
    packs.extend(items_pack);
    let terrain = TerrainTextures::from_packs(&packs);
    if let Some(resources) = terrain.items() {
        resources.set_custom_items(Arc::new(items));
    }
    progress.set(Step::Files);
    let jar = crate::resources::official::find_verified_jar().ok();
    let files = GameFiles::new(pack_files, jar)
        .with_features(features)
        .with_custom_guis(guis);
    let custom = terrain
        .animation_layout()
        .map(|(context, layer_size)| {
            let layer_of = |key: &str| context.atlas.layer(key);
            custom_animations::load(&files, &packs, &layer_of, layer_size)
        })
        .unwrap_or_default();
    (terrain, files, custom)
}

/// Starts the thread that pings the saved servers. Without it the list still
/// works and every row stays "pinging".
fn start_pinger() -> Option<Pinger> {
    match Pinger::new() {
        Ok(pinger) => Some(pinger),
        Err(error) => {
            tracing::warn!(%error, "server pings are off");
            None
        }
    }
}

/// A number that differs from start to start, for the title splash.
fn menu_seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

/// Uploads the GUI atlas to the renderer, if available.
pub(super) fn upload_gui_atlas(
    renderer: &mut Renderer,
    gui_atlas: Option<&cindral_assets::GuiAtlas>,
) {
    if let Some(atlas) = gui_atlas {
        if !atlas.is_empty() {
            match renderer.set_gui_atlas(atlas.width(), atlas.height(), atlas.pixels()) {
                Ok(()) => tracing::info!(
                    regions = atlas.regions().len(),
                    "{}x{} GUI texture atlas uploaded",
                    atlas.width(),
                    atlas.height()
                ),
                Err(error) => tracing::error!(
                    %error,
                    "GUI texture atlas rejected; the previous GUI textures stay"
                ),
            }
        }
    }
}

/// Uploads the sun, moon and End sky pictures of the verified jar.
pub(super) fn upload_sky_textures(renderer: &mut Renderer, files: &GameFiles) {
    renderer.set_sky_textures(&load_sky_textures(files));
    if let Some(forcefield) = load_border_texture(files) {
        renderer.set_border_texture(&forcefield);
    }
}

/// Loads the pack-defined sky layers and uploads their pictures. Called with
/// the sky pictures, at startup and on every resource reload.
pub(super) fn upload_custom_sky(renderer: &mut Renderer, files: &GameFiles) -> CustomSky {
    let (sky, images) = CustomSky::load(files);
    renderer.set_custom_sky_textures(&images);
    sky
}

/// Uploads the enchantment glint texture to the renderer, if available.
pub(super) fn upload_glint_texture(
    renderer: &mut Renderer,
    glint: Option<&cindral_assets::GlintTexture>,
) {
    if let Some(glint) = glint {
        renderer.set_glint_texture(glint.width, glint.height, &glint.pixels);
    }
}

/// The menu flow, with the splash of the user's `splashes.txt` when there is
/// one (the title screen keeps its original list otherwise).
fn new_menu(last_server: String) -> MenuFlow {
    let seed = menu_seed();
    let mut menu = MenuFlow::new(
        ServerBook::open_default(),
        start_pinger(),
        last_server,
        seed,
    );
    menu.set_accounts(AccountsFlow::open_default());
    let lines = crate::screens::splash::load_lines();
    if let Some(splash) =
        crate::screens::splash::choose(&lines, crate::screens::splash::today(), seed)
    {
        menu.set_splash(splash);
    }
    menu
}

impl App {
    pub(super) fn new(
        args: Args,
        options: Options,
        options_path: Option<std::path::PathBuf>,
        lang: Arc<Lang>,
    ) -> Result<Self, ClientError> {
        // Launch background thread to decode heavy assets without blocking the main event loop
        let packs = pack_paths(
            &args.resource_packs,
            &options,
            packs_folder().as_deref(),
            None,
        );
        let features = PackFeatures::from_quality(&options.video.quality);
        // The pack loader reads options that shape the loaded resources.
        crate::meshing::settings::store(&options.video);
        let (asset_tx, asset_rx) = std::sync::mpsc::channel();
        let progress = LoadProgress::default();
        let loader_progress = progress.clone();
        let loader_thread = std::thread::Builder::new()
            .name("AssetLoader".to_string())
            .spawn(move || {
                let (terrain, files, custom_animations) =
                    load_packs_and_terrain(&packs, features, &loader_progress);
                loader_progress.set(Step::Font);
                let font = ui_font::load_with(&files);
                let pictures = DecodedPictures::load(&files);
                let _ = asset_tx.send(AsyncLoadedAssets {
                    font,
                    terrain,
                    files,
                    custom_animations,
                    pictures,
                });
            });
        if let Err(error) = loader_thread {
            tracing::error!(%error, "Failed to spawn background asset loader thread");
        }

        let font = LoadedFont {
            atlas: FontAtlas::fallback(),
            unicode: None,
            source: crate::resources::ui_font::FontSource::Fallback,
        };
        // Offline tests and scripted runs (`--no-audio`) never download.
        let first_run = if args.no_audio {
            None
        } else {
            crate::first_run::FirstRun::start_if_needed()
        };
        let updater = (!args.no_audio).then(crate::updater::Updater::start);
        let audio = if args.no_audio {
            AudioBridge::disabled()
        } else {
            AudioBridge::start(&options.sound)
        };
        crate::screens::tooltip::set_advanced(options.controls.advanced_tooltips);
        let last_server = options.multiplayer.last_server.clone();
        let ui_viewport = Rect::new(0.0, 0.0, args.width as f32, args.height as f32);
        Ok(Self {
            args,
            options_menu: OptionsMenu {
                path: options_path,
                saved: options.clone(),
                screen: OptionsScreen::new(),
                outcome: OptionsOutcome::default(),
                video_applier: VideoApplier::new(options.video.fullscreen),
                origin: Screen::GameMenu,
            },
            discord: crate::discord::Presence::spawn(),
            first_run,
            updater,
            window_shown: false,
            splash: Splash::new(progress),
            stutters: crate::debug::stutter::StutterLog::new(std::time::Instant::now()),
            hitches: crate::debug::stutter::HitchLog::default(),
            lang,
            mods: ModsRuntime::new(&options.mods),
            mods_memory: (0, None),
            options,
            audio,
            window: None,
            renderer: None,
            panorama: None,
            camera: Camera::default(),
            overlay: Overlay::new(1280, 720),
            show_overlay: true,
            input: InputTracking {
                keys: HashSet::new(),
                buttons: HashSet::new(),
                mouse_captured: false,
                ctrl_held: false,
                shift_held: false,
                pointer: (0.0, 0.0),
                mouse: crate::input::mouse_look::MouseSteps::default(),
                last: InputState::default(),
                last_time: Instant::now(),
            },
            world: None,
            session: None,
            menu: new_menu(last_server),
            presses: PendingPresses {
                select_slot: None,
                hotbar_keys: [0; 9],
                break_block: false,
                place_block: false,
                pick_block: false,
                drop_item: None,
                toggle_perspective: false,
                screenshot: false,
                respawn: false,
            },
            death_started: None,
            screen: Screen::Loading,
            assets: Assets {
                font,
                receiver: Some(asset_rx),
                pending: None,
                files: GameFiles::default(),
                leaf_layers: Vec::new(),
                ui_atlas: None,
                font_choice: FontChoice::Game,
                brand_atlas: None,
                gui_atlas: None,
                items: None,
                hand_layers: ItemLayers::default(),
                texture_animator: TextureAnimator::default(),
                entity_textures: None,
                map_textures: None,
                custom_sky: CustomSky::default(),
                world_colors: Default::default(),
            },
            chat: ChatUi {
                input: ChatInput::new(""),
                sent: SentHistory::new(),
                scroll: 0,
                opened: Instant::now(),
                seen_completions: 0,
                hover: None,
                line: None,
            },
            frame: FrameScratch {
                hud: HudState::default(),
                hud_revision: 0,
                hud_chat: None,
                entities: Vec::new(),
                dynamic_lights: Default::default(),
                particle_sprites: Vec::new(),
                nametags: Vec::new(),
                placed_tags: Vec::new(),
                moving_blocks_mesh: cindral_mesh::Mesh::default(),
                last_hand: None,
            },
            cpu: CpuSplit::default(),
            zoom: Zoom::default(),
            focused: true,
            hud_cache: HudCache::default(),
            creative_input: CreativeInput::new(),
            container_was_open: false,
            ui: Ui::new(ui_viewport),
            show_f3: false,
            f3_chord_used: false,
            show_hitboxes: false,
            show_chunk_borders: false,
            debug_screen: DebugScreen::new(),
            game_menu: GameMenu::new(),
            menu_action: None,
            menu_command: None,
            ui_input: UiInputTranslator::new(),
            inventory_input: InventoryInput::new(),
            server_icons_revision: u64::MAX,
            dials: DialAnimator::default(),
            world_items: WorldItems::default(),
            entity_icons: EntityIcons::default(),
            portrait: None,
            screen_book: None,
            anvil_field: AnvilField::default(),
            text_screens: TextScreens::default(),
            server_pack: server_pack_control::ServerPackRuntime::default(),
            clock: FrameClock::new(Instant::now()),
            exit_error: None,
        })
    }

    /// Checks if the background worker has finished decoding assets.
    ///
    /// Nothing is applied before the loading screen is on the screen: on a
    /// fast disk the assets are ready before the first frame, and applying
    /// them then went straight to the title screen behind a hidden window.
    /// Once they arrive, one frame shows the upload step, then the upload
    /// (which holds the window for a moment) runs and the screen fades out.
    pub(super) fn poll_asset_loading(&mut self) {
        if self.renderer.is_none() || !self.window_shown {
            return;
        }
        let now = Instant::now();
        if self.splash.shown_at.is_none() {
            self.splash.shown_at = Some(now);
        }
        if let Some(pending) = self.assets.pending.take() {
            self.assets.pending = if self.splash.may_leave(now) {
                self.upload_stage(pending)
            } else {
                Some(pending)
            };
            return;
        }
        let Some(rx) = self.assets.receiver.as_ref() else {
            return;
        };
        match rx.try_recv() {
            Ok(assets) => {
                self.assets.receiver = None;
                self.splash.progress.set(Step::Upload);
                self.assets.pending = Some(PendingUpload::Decoded(assets));
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                tracing::warn!("Asset loader thread disconnected unexpectedly");
                self.assets.receiver = None;
                if self.screen == Screen::Loading {
                    self.screen = Screen::Title;
                    self.splash.fade_from = Some(Instant::now());
                }
            }
        }
    }

    /// Uploads one stage of `pending` and returns what is left.
    fn upload_stage(&mut self, pending: PendingUpload) -> Option<PendingUpload> {
        match pending {
            PendingUpload::Decoded(assets) => {
                let staged = self.upload_terrain(assets);
                self.splash.progress.set(Step::World);
                staged
            }
            PendingUpload::Terrain { terrain, rest } => {
                self.finish_upload(&terrain, rest);
                None
            }
        }
    }

    /// The first upload stage: the game font and the block textures, the
    /// largest part (mipmaps of a thousand layers).
    fn upload_terrain(&mut self, assets: AsyncLoadedAssets) -> Option<PendingUpload> {
        let renderer = self.renderer.as_mut()?;

        self.assets.font = assets.font;
        match ui_font::install(renderer, &self.assets.font.atlas) {
            Ok(installed) => {
                // The game font is bound now: the theme font is swapped in
                // again by the next frame if the theme wants it.
                self.assets.font_choice = FontChoice::Game;
                self.assets.ui_atlas = Some(Arc::new(installed.atlas));
                self.hud_cache.invalidate();
            }
            Err(error) => {
                tracing::warn!(%error, "failed to install official font atlas; keeping fallback");
            }
        }
        // The brand font was installed with the renderer and does not depend
        // on the packs: rasterizing it again here only held the window.

        renderer.set_mipmap_levels(u32::from(self.options.video.mipmap_levels));
        let terrain = assets.terrain.install(renderer);
        Some(PendingUpload::Terrain {
            terrain,
            rest: AfterTerrain {
                files: assets.files,
                custom_animations: assets.custom_animations,
                pictures: assets.pictures,
            },
        })
    }

    /// The second upload stage: the menus, the sky, the entities and the
    /// world view; then the loading screen fades into the title screen.
    fn finish_upload(&mut self, terrain: &InstalledTerrain, assets: AfterTerrain) {
        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };
        let gui_atlas = crate::resources::official::grow_atlas(terrain.gui_atlas(), &[]);
        upload_gui_atlas(renderer, Some(&gui_atlas));
        upload_glint_texture(renderer, terrain.glint_texture());
        self.assets.files = assets.files;
        let pictures = assets.pictures;
        renderer.set_sky_textures(&pictures.sky);
        if let Some(border) = &pictures.border {
            renderer.set_border_texture(border);
        }
        renderer.set_custom_sky_textures(&pictures.custom_sky_images);
        self.assets.custom_sky = pictures.custom_sky;
        self.assets.world_colors =
            crate::resources::custom_colors::WorldColors::upload(renderer, &self.assets.files);
        self.panorama = pictures
            .panorama
            .as_ref()
            .and_then(|faces| renderer.upload_panorama(faces));
        self.assets.gui_atlas = Some(gui_atlas);
        self.server_icons_revision = u64::MAX;
        self.assets.items = terrain.items();
        self.dials = DialAnimator::new(terrain.dials());
        self.assets.leaf_layers = terrain.leaf_layers();
        self.assets.texture_animator = TextureAnimator::for_terrain(
            terrain,
            super::options_control::animation_filter(&self.options.video.animations),
        )
        .with_custom(assets.custom_animations);

        match WorldView::new(terrain, worker_count(self.options.video.mesh_threads)) {
            Ok(world) => self.world = Some(world),
            Err(error) => {
                tracing::error!(%error, "WorldView::new failed");
            }
        }

        self.assets.entity_textures = Some(EntityTextures::load_with_font(
            renderer,
            self.assets.ui_atlas.as_ref(),
            self.assets.files.clone(),
        ));
        self.assets.map_textures = Some(MapTextures::load(renderer, &self.assets.files));

        self.splash.progress.set(Step::Done);
        if self.screen == Screen::Loading {
            self.splash.fade_from = Some(Instant::now());
            if self.args.server().is_some() {
                self.connect_from_arguments();
            } else {
                self.screen = Screen::Title;
            }
        }
    }
}
