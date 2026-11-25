// SPDX-License-Identifier: MIT OR Apache-2.0

//! `cindral-client` entry point: choose the windowed client, the offscreen
//! screenshot, the headless scripted session, the server-less UI demo or the
//! server-less M5 block demo.

// A release build is a windowed app: opened from Explorer or a shortcut it
// shows no console. Run from a terminal, it writes to that terminal
// (`attach_parent_console`).
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use std::process::ExitCode;

use cindral_client::args::{Args, Mode, USAGE};
use cindral_client::demos::{bench_scene, block_demo, entity_demo, hud_demo, ui_demo};
use cindral_client::{audio_bridge, headless, launch_session, screenshot, setup_cli, window};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

#[global_allocator]
static ALLOCATOR: cindral_core::profiling::CountingAllocator =
    cindral_core::profiling::CountingAllocator::new();

fn main() -> ExitCode {
    attach_parent_console();

    // Kept alive for the whole run: dropping it stops the puffin stream.
    let _profiler = cindral_core::profiling::serve();
    // A 1 ms timer for the whole run, so the game thread's 50 ms ticks start
    // on time on Windows (timer_resolution.rs).
    let _timer = cindral_client::timer_resolution::TimerResolution::raise();

    let words: Vec<String> = std::env::args().skip(1).collect();
    if words.iter().any(|word| word == "--help" || word == "-h") {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let parsed = launch_session::split(&words)
        .and_then(|(rest, session)| Args::parse(&rest).map(|args| Args { session, ..args }));
    let args = match parsed {
        Ok(args) => args,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };
    // Only the game writes the log file: a benchmark, a demo or the
    // installer's asset check would replace the session a player wants to
    // send.
    init_logging(matches!(args.mode(), Mode::Windowed));

    if args.profile {
        cindral_core::profiling::set_enabled(true);
    }

    // Kept alive for the whole run: dropping it stops the puffin stream.
    let _profiler = cindral_core::profiling::serve();

    let result = match args.mode() {
        Mode::SetupAssets => setup_cli::run(),
        Mode::AudioProbe => audio_bridge::probe::run(),
        Mode::UiDemo => ui_demo::run(&args),
        Mode::HudDemo => hud_demo::run(&args),
        Mode::BlockDemo => block_demo::run(&args),
        Mode::EntityDemo => entity_demo::run(&args),
        Mode::Headless => headless::run(&args),
        Mode::BenchScene => bench_scene::run(&args),
        Mode::Screenshot => screenshot::run(&args),
        Mode::Windowed => window::run(args.clone()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("cindral: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Writes the output of a release build to the terminal it was started from,
/// if any: a windowed app has no console of its own. Output already sent to a
/// pipe or a file stays there.
#[cfg(windows)]
#[allow(unsafe_code)]
fn attach_parent_console() {
    use windows_sys::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
    if cfg!(debug_assertions) {
        return;
    }
    // SAFETY: a plain Win32 call without pointers; it fails harmlessly when
    // the parent has no console or this process already has one.
    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

#[cfg(not(windows))]
fn attach_parent_console() {}

/// The log goes to the terminal (warnings, or `RUST_LOG`) and, for the
/// game (`to_file`), to `logs/latest.log` in the data folder: information
/// too (setbacks, the frame summary, what was loaded). The four sessions
/// before it are kept as `previous.log`, `previous-2.log` .. `previous-4.log`.
/// A release build has no console, so the file is what a player can send
/// when something goes wrong.
fn init_logging(to_file: bool) {
    let terminal = tracing_subscriber::fmt::layer()
        .with_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")));
    let file = to_file
        .then(cindral_assets::cache::data_dir)
        .flatten()
        .and_then(|dir| {
            let folder = dir.join("logs");
            std::fs::create_dir_all(&folder).ok()?;
            rotate_logs(&folder);
            std::fs::File::create(folder.join("latest.log"))
                .ok()
                .or_else(|| new_session_log(&folder))
        });
    let file = file.map(|file| {
        tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_writer(std::sync::Mutex::new(file))
            .with_filter(EnvFilter::new(
                "info,wgpu=warn,wgpu_core=warn,wgpu_hal=warn,naga=warn,rustls=warn,hyper=warn,reqwest=warn",
            ))
    });
    tracing_subscriber::registry()
        .with(terminal)
        .with(file)
        .init();
}

/// A log file of its own for this session, `session-<seconds>.log`, when
/// `latest.log` cannot be replaced: some antivirus programs (Kaspersky's
/// AntiCryptor) let an unknown program create files but not rename or
/// overwrite them, and the session then went without any log.
fn new_session_log(folder: &std::path::Path) -> Option<std::fs::File> {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(folder.join(format!("session-{seconds}.log")))
        .ok()
}

/// Shifts the kept sessions by one: `previous-3` becomes `previous-4`, ..,
/// `previous` becomes `previous-2` and `latest` becomes `previous`; the
/// oldest is dropped.
fn rotate_logs(folder: &std::path::Path) {
    const KEPT: u32 = 4;
    let name = |index: u32| match index {
        0 => "latest.log".to_owned(),
        1 => "previous.log".to_owned(),
        n => format!("previous-{n}.log"),
    };
    for index in (0..KEPT).rev() {
        let from = folder.join(name(index));
        if from.exists() {
            let _ = std::fs::rename(&from, folder.join(name(index + 1)));
        }
    }
}
