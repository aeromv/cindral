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
use tracing_subscriber::EnvFilter;

#[global_allocator]
static ALLOCATOR: cindral_core::profiling::CountingAllocator =
    cindral_core::profiling::CountingAllocator::new();

fn main() -> ExitCode {
    attach_parent_console();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .init();

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
