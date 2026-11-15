// SPDX-License-Identifier: MIT OR Apache-2.0

//! A once-a-second performance log of a real game (`--perf-log PATH`).
//!
//! The bench scene has no HUD, entities, particles or chunk streaming; this
//! log is how a weak machine shows where its frames go on a real server. Each
//! line covers one second: the frame rate, the mean and worst frame, the
//! window thread's CPU time per stage (the F3 `CPU` stages, not smoothed),
//! the GPU time per group when the adapter has timestamps, the parts of the
//! `scene` stage ([`SCENE_PARTS`]) and what the frame held (entities,
//! particles, name tags, block entities). The file is plain text, one header
//! line then one row per second, so it can be read without a tool.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::Instant;

use cindral_render::{GpuTimings, GROUP_NAMES};

use crate::cpu_split::STAGE_NAMES;

/// The parts of the `scene` stage, in the order the window runs them.
pub const SCENE_PARTS: [&str; 7] = [
    "particles",
    "overlays",
    "block_entities",
    "entity_items",
    "entities",
    "hand_and_items",
    "portrait",
];

/// The parts of the `hud` stage, in the order the window runs them.
pub const HUD_PARTS: [&str; 7] = [
    "state",
    "build",
    "widgets",
    "tooltip",
    "tags",
    "screens",
    "end_frame",
];

/// What one frame's `hud` stage did.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct HudDetail {
    /// Microseconds per part, in the order of [`HUD_PARTS`].
    pub us: [f32; HUD_PARTS.len()],
    /// Whether the HUD draw list was built again (the cache missed).
    pub rebuilt: bool,
}

/// A clock for the parts of a stage: each lap ends one part.
#[derive(Debug)]
pub struct PartClock<const N: usize> {
    last: Instant,
    us: [f32; N],
}

impl<const N: usize> PartClock<N> {
    /// Starts timing now.
    #[must_use]
    pub fn start() -> Self {
        Self {
            last: Instant::now(),
            us: [0.0; N],
        }
    }

    /// Ends part `index`: the time since the last lap is its.
    pub fn lap(&mut self, index: usize) {
        let now = Instant::now();
        if let Some(slot) = self.us.get_mut(index) {
            *slot += now.duration_since(self.last).as_secs_f32() * 1e6;
        }
        self.last = now;
    }

    /// The times of the parts, in microseconds.
    #[must_use]
    pub fn parts(&self) -> [f32; N] {
        self.us
    }
}

/// What one frame's `scene` stage did: the time of each part and what the
/// frame held.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SceneDetail {
    /// Microseconds per part, in the order of [`SCENE_PARTS`].
    pub us: [f32; SCENE_PARTS.len()],
    /// Entities in the frame (before culling).
    pub entities: usize,
    /// Particle sprites drawn.
    pub particles: usize,
    /// Name tags drawn.
    pub tags: usize,
    /// Block entities loaded (signs, chests, skulls, banners...).
    pub block_entities: usize,
}

/// A clock for the parts of the `scene` stage.
#[derive(Debug)]
pub struct SceneClock {
    last: Instant,
    detail: SceneDetail,
}

impl SceneClock {
    /// Starts timing the stage now.
    #[must_use]
    pub fn start() -> Self {
        Self {
            last: Instant::now(),
            detail: SceneDetail::default(),
        }
    }

    /// Ends part `index` of [`SCENE_PARTS`]: the time since the last lap is its.
    pub fn lap(&mut self, index: usize) {
        let now = Instant::now();
        if let Some(slot) = self.detail.us.get_mut(index) {
            *slot += now.duration_since(self.last).as_secs_f32() * 1e6;
        }
        self.last = now;
    }

    /// The times so far, with the counts of the frame.
    #[must_use]
    pub fn finish(
        self,
        entities: usize,
        particles: usize,
        tags: usize,
        block_entities: usize,
    ) -> SceneDetail {
        SceneDetail {
            entities,
            particles,
            tags,
            block_entities,
            ..self.detail
        }
    }
}

/// One second of frames, written as one row.
#[derive(Debug, Default)]
struct Window {
    frames: u32,
    frame_sum_ms: f32,
    frame_max_ms: f32,
    stage_sum_us: [f32; STAGE_NAMES.len()],
    gpu_sum_ms: [f32; GROUP_NAMES.len()],
    gpu_samples: u32,
    scene_sum_us: [f32; SCENE_PARTS.len()],
    /// Sums of entities, particles, tags and block entities.
    counts_sum: [u64; 4],
    hud_sum_us: [f32; HUD_PARTS.len()],
    /// Frames whose HUD draw list was built again.
    hud_rebuilds: u32,
    /// The packets' wait for the game thread: mean and longest (ms), count.
    packet_lag: (f32, f32, u64),
    /// How late the game ticks started: mean and longest (ms).
    tick_late: (f32, f32),
    /// Server corrections of the player's position in the second.
    corrections: u64,
}

/// The open log and the second being collected.
#[derive(Debug)]
pub struct PerfLog {
    out: BufWriter<File>,
    /// The correction count at the last row.
    corrections_seen: u64,
    started: Instant,
    window_start: Instant,
    window: Window,
}

impl PerfLog {
    /// Creates (or truncates) the log at `path` and writes its header.
    ///
    /// # Errors
    /// The file cannot be created or written.
    pub fn create(path: &Path, now: Instant) -> std::io::Result<Self> {
        let mut out = BufWriter::new(File::create(path)?);
        writeln!(out, "{}", header())?;
        out.flush()?;
        Ok(Self {
            out,
            corrections_seen: crate::game::SERVER_CORRECTIONS
                .load(std::sync::atomic::Ordering::Relaxed),
            started: now,
            window_start: now,
            window: Window::default(),
        })
    }

    /// Adds one frame; writes a row when a second has passed since the last.
    ///
    /// `stages_us` are this frame's CPU stage times in microseconds.
    ///
    /// # Errors
    /// The row cannot be written.
    pub fn record(
        &mut self,
        now: Instant,
        frame_ms: f32,
        stages_us: [f32; STAGE_NAMES.len()],
        gpu: Option<GpuTimings>,
        (scene, hud): (&SceneDetail, &HudDetail),
        lag: Option<&crate::game::PacketLag>,
    ) -> std::io::Result<()> {
        let window = &mut self.window;
        window.frames += 1;
        window.frame_sum_ms += frame_ms;
        window.frame_max_ms = window.frame_max_ms.max(frame_ms);
        for (sum, stage) in window.stage_sum_us.iter_mut().zip(stages_us) {
            *sum += stage;
        }
        if let Some(gpu) = gpu {
            for (sum, ms) in window.gpu_sum_ms.iter_mut().zip(gpu.ms) {
                *sum += ms;
            }
            window.gpu_samples += 1;
        }
        for (sum, part) in window.scene_sum_us.iter_mut().zip(scene.us) {
            *sum += part;
        }
        let counts = [
            scene.entities,
            scene.particles,
            scene.tags,
            scene.block_entities,
        ];
        for (sum, count) in window.counts_sum.iter_mut().zip(counts) {
            *sum += count as u64;
        }
        for (sum, part) in window.hud_sum_us.iter_mut().zip(hud.us) {
            *sum += part;
        }
        window.hud_rebuilds += u32::from(hud.rebuilt);
        let span = now.duration_since(self.window_start).as_secs_f32();
        if span < 1.0 {
            return Ok(());
        }
        if let Some(lag) = lag {
            self.window.packet_lag = lag.take();
            self.window.tick_late = lag.take_ticks();
            let total = crate::game::SERVER_CORRECTIONS.load(std::sync::atomic::Ordering::Relaxed);
            self.window.corrections = total.saturating_sub(self.corrections_seen);
            self.corrections_seen = total;
        }
        let row = row(
            &std::mem::take(&mut self.window),
            span,
            now.duration_since(self.started).as_secs_f32(),
        );
        self.window_start = now;
        writeln!(self.out, "{row}")?;
        self.out.flush()
    }
}

/// The names of the columns.
fn header() -> String {
    let names = |prefix: &str, names: &[&str]| {
        names
            .iter()
            .map(|name| format!("{prefix}{name}_ms"))
            .collect::<Vec<_>>()
            .join(" ")
    };
    format!(
        "t_s fps frame_mean_ms frame_max_ms {} cpu_total_ms gpu_total_ms {} {} entities particles tags block_entities {} hud_rebuilds packet_wait_mean_ms packet_wait_max_ms packets tick_late_mean_ms tick_late_max_ms server_corrections",
        names("cpu_", &STAGE_NAMES),
        names("gpu_", &GROUP_NAMES),
        names("scene_", &SCENE_PARTS),
        names("hud_", &HUD_PARTS),
    )
}

/// The text of one row: see [`header`].
fn row(window: &Window, span_s: f32, t_s: f32) -> String {
    let frames = window.frames.max(1) as f32;
    let per_frame_ms =
        |sums: &[f32]| -> Vec<f32> { sums.iter().map(|sum| sum / frames / 1000.0).collect() };
    let stages = per_frame_ms(&window.stage_sum_us);
    let cpu_total: f32 = stages.iter().sum();
    let gpu: Vec<f32> = if window.gpu_samples == 0 {
        vec![f32::NAN; GROUP_NAMES.len()]
    } else {
        window
            .gpu_sum_ms
            .iter()
            .map(|sum| sum / window.gpu_samples as f32)
            .collect()
    };
    let gpu_total: f32 = gpu.iter().sum();
    let scene = per_frame_ms(&window.scene_sum_us);
    let hud = per_frame_ms(&window.hud_sum_us);
    let format = |values: &[f32]| {
        values
            .iter()
            .map(|value| format!("{value:.3}"))
            .collect::<Vec<_>>()
            .join(" ")
    };
    let counts: Vec<String> = window
        .counts_sum
        .iter()
        .map(|sum| format!("{:.0}", *sum as f32 / frames))
        .collect();
    format!(
        "{t_s:.1} {:.0} {:.3} {:.3} {} {cpu_total:.3} {gpu_total:.3} {} {} {} {} {} {:.2} {:.2} {} {:.2} {:.2} {}",
        window.frames as f32 / span_s,
        window.frame_sum_ms / frames,
        window.frame_max_ms,
        format(&stages),
        format(&gpu),
        format(&scene),
        counts.join(" "),
        format(&hud),
        window.hud_rebuilds,
        window.packet_lag.0,
        window.packet_lag.1,
        window.packet_lag.2,
        window.tick_late.0,
        window.tick_late.1,
        window.corrections,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_row_averages_the_frames_of_its_second() {
        let window = Window {
            frames: 4,
            frame_sum_ms: 10.0,
            frame_max_ms: 4.0,
            stage_sum_us: [400.0, 800.0, 1200.0, 1600.0],
            gpu_sum_ms: [4.0, 2.0, 0.0, 0.0, 0.0],
            gpu_samples: 2,
            scene_sum_us: [400.0, 0.0, 800.0, 0.0, 4000.0, 0.0, 0.0],
            counts_sum: [40, 8, 4, 0],
            hud_sum_us: [0.0, 400.0, 0.0, 0.0, 0.0, 0.0, 800.0],
            hud_rebuilds: 1,
            packet_lag: (2.5, 40.0, 900),
            tick_late: (1.0, 4.0),
            corrections: 2,
        };
        assert_eq!(
            row(&window, 1.0, 3.0),
            "3.0 4 2.500 4.000 0.100 0.200 0.300 0.400 1.000 3.000 2.000 1.000 0.000 0.000 0.000 \
             0.100 0.000 0.200 0.000 1.000 0.000 0.000 10 2 1 0 \
             0.000 0.100 0.000 0.000 0.000 0.000 0.200 1 2.50 40.00 900 1.00 4.00 2"
        );
    }

    #[test]
    fn the_header_names_every_column_of_a_row() {
        let window = Window {
            frames: 1,
            ..Window::default()
        };
        let columns = row(&window, 1.0, 1.0).split(' ').count();
        assert_eq!(header().split(' ').count(), columns);
    }

    #[test]
    fn the_clock_adds_each_part_to_its_slot() {
        let mut clock = SceneClock::start();
        clock.lap(4);
        clock.lap(4);
        clock.lap(99);
        let detail = clock.finish(3, 2, 1, 5);
        assert!(detail.us[4] >= 0.0);
        assert_eq!(detail.us[0], 0.0);
        assert_eq!(
            (
                detail.entities,
                detail.particles,
                detail.tags,
                detail.block_entities
            ),
            (3, 2, 1, 5)
        );
    }

    #[test]
    fn the_log_writes_a_header_then_one_row_per_second() {
        let dir = std::env::temp_dir().join(format!("cindral-perf-log-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("perf.log");
        let start = Instant::now();
        let mut log = PerfLog::create(&path, start).unwrap();
        for frame in 1..=5 {
            let now = start + std::time::Duration::from_millis(300 * frame);
            log.record(
                now,
                3.0,
                [100.0; 4],
                None,
                (&SceneDetail::default(), &HudDetail::default()),
                None,
            )
            .unwrap();
        }
        drop(log);
        let text = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2, "{text}");
        assert!(lines[0].starts_with("t_s fps"));
        assert!(lines[1].starts_with("1.2 3 3.000"), "{}", lines[1]);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
