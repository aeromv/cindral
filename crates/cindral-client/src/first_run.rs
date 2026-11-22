// SPDX-License-Identifier: MIT OR Apache-2.0

//! The game files on the first launch.
//!
//! Cindral ships no Mojang file. When the 1.8.8 jar or the sounds are not on
//! this machine yet, the window starts anyway and a background thread fetches
//! them from Mojang, checking every file against Mojang's SHA-1, as
//! `--setup-assets` does. The title screen shows the progress; when the
//! download ends the window reloads its textures and starts the sounds.

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use cindral_assets::cache::AssetCache;
use cindral_assets::http::Endpoints;
use cindral_assets::progress::Progress;
use cindral_assets::setup::{find_existing_jar, run_setup};
use cindral_assets::SoundAssets;

/// How many files are fetched at once.
const CONCURRENCY: usize = 8;

const RUNNING: u8 = 0;
const DONE: u8 = 1;
const FAILED: u8 = 2;

/// Where the first-launch download stands.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FirstRunState {
    /// Still fetching; the completed fraction, `0.0` to `1.0`.
    Running(f64),
    /// Every file is on disk and verified.
    Done,
    /// The download stopped (no network, Mojang unreachable); the game runs
    /// with what it has and tries again on the next launch.
    Failed,
}

/// The background download of the game files.
#[derive(Debug)]
pub struct FirstRun {
    progress: Progress,
    state: Arc<AtomicU8>,
}

impl FirstRun {
    /// Starts the download when the jar or the sounds are missing; `None`
    /// when everything is already here.
    #[must_use]
    pub fn start_if_needed() -> Option<Self> {
        let cache = AssetCache::in_data_dir().ok()?;
        let has_jar = find_existing_jar(&cache).is_some();
        let has_sounds = SoundAssets::open().is_some();
        if has_jar && has_sounds {
            return None;
        }
        tracing::info!(has_jar, has_sounds, "downloading the game files");
        let progress = Progress::new();
        let state = Arc::new(AtomicU8::new(RUNNING));
        let (thread_progress, thread_state) = (progress.clone(), Arc::clone(&state));
        let spawned = std::thread::Builder::new()
            .name("cindral-first-run".to_string())
            .spawn(move || {
                let ok = download(cache, &thread_progress);
                thread_state.store(if ok { DONE } else { FAILED }, Ordering::Release);
            });
        if let Err(error) = spawned {
            tracing::warn!(%error, "could not start the game files download");
            return None;
        }
        Some(Self { progress, state })
    }

    /// Where the download stands now.
    #[must_use]
    pub fn state(&self) -> FirstRunState {
        match self.state.load(Ordering::Acquire) {
            DONE => FirstRunState::Done,
            FAILED => FirstRunState::Failed,
            _ => FirstRunState::Running(self.progress.snapshot().fraction()),
        }
    }
}

/// The line the title screen shows while the download runs.
#[must_use]
pub fn status_line(fraction: f64) -> String {
    let percent = (fraction * 100.0).floor().clamp(0.0, 100.0);
    format!("Downloading the game files from Mojang... {percent:.0}%")
}

/// Fetches the jar and the asset objects; whether it all succeeded.
fn download(cache: AssetCache, progress: &Progress) -> bool {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            tracing::warn!(%error, "no runtime for the game files download");
            return false;
        }
    };
    let result = runtime.block_on(run_setup(
        Endpoints::official(),
        Arc::new(cache),
        CONCURRENCY,
        progress,
        |_| {},
    ));
    match result {
        Ok((_, report)) => {
            tracing::info!(
                downloaded = report.downloaded,
                cached = report.cached,
                "game files ready"
            );
            true
        }
        Err(error) => {
            tracing::warn!(%error, "the game files download failed");
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_status_line_shows_a_whole_percentage() {
        assert_eq!(
            status_line(0.427),
            "Downloading the game files from Mojang... 42%"
        );
        assert_eq!(
            status_line(1.5),
            "Downloading the game files from Mojang... 100%"
        );
    }
}
