// SPDX-License-Identifier: MIT OR Apache-2.0

//! A once-a-minute line in the log about the frame times: the average rate,
//! the slowest frame and how many frames stuttered. With the setbacks the
//! game thread logs, it lets a player send one file that shows what went
//! wrong on a server.

use std::time::{Duration, Instant};

/// How often a summary is written.
const PERIOD: Duration = Duration::from_secs(60);
/// A frame longer than this is a visible stutter.
const STUTTER: Duration = Duration::from_millis(50);

/// A frame longer than this is logged on its own, with where its time went.
const HITCH: Duration = Duration::from_millis(100);
/// At most one hitch line this often, so a bad minute does not flood the log.
const HITCH_GAP: Duration = Duration::from_secs(2);

/// One log line per long frame (a hitch): how long it took and which stage
/// of the frame took it. The minute summary only says that hitches happened;
/// this says what they were.
#[derive(Debug, Default)]
pub(crate) struct HitchLog {
    last: Option<Instant>,
}

impl HitchLog {
    /// The line for a `frame` that was a hitch, `None` for a normal one or
    /// within [`HITCH_GAP`] of the previous line. `stages` are the drawing
    /// stages of that frame in microseconds, in the order of `names`; the
    /// rest of the frame was spent outside the drawing (window events,
    /// waiting for the GPU or the frame cap).
    pub(crate) fn record(
        &mut self,
        frame: Duration,
        now: Instant,
        (names, stages): (&[&str], &[f32]),
    ) -> Option<String> {
        if frame <= HITCH
            || self
                .last
                .is_some_and(|last| now.duration_since(last) < HITCH_GAP)
        {
            return None;
        }
        self.last = Some(now);
        let drawing: f32 = stages.iter().sum::<f32>() / 1000.0;
        let parts: Vec<String> = names
            .iter()
            .zip(stages)
            .map(|(name, micros)| format!("{name} {:.0}", micros / 1000.0))
            .collect();
        let total = frame.as_secs_f32() * 1000.0;
        Some(format!(
            "hitch: a frame took {total:.0} ms ({}, outside drawing {:.0}, in ms)",
            parts.join(", "),
            (total - drawing).max(0.0)
        ))
    }
}

/// The frame times since the last summary.
#[derive(Debug)]
pub(crate) struct StutterLog {
    since: Instant,
    frames: u32,
    total: Duration,
    slowest: Duration,
    stutters: u32,
}

impl StutterLog {
    pub(crate) fn new(now: Instant) -> Self {
        Self {
            since: now,
            frames: 0,
            total: Duration::ZERO,
            slowest: Duration::ZERO,
            stutters: 0,
        }
    }

    /// Records one frame and returns the summary line once a period is over.
    pub(crate) fn record(&mut self, frame: Duration, now: Instant) -> Option<String> {
        self.frames += 1;
        self.total += frame;
        self.slowest = self.slowest.max(frame);
        if frame > STUTTER {
            self.stutters += 1;
        }
        if now.duration_since(self.since) < PERIOD {
            return None;
        }
        let fps = f64::from(self.frames) / self.total.as_secs_f64().max(1e-6);
        let line = format!(
            "frames: {fps:.0} fps on average, slowest {:.0} ms, {} over {} ms",
            self.slowest.as_secs_f64() * 1000.0,
            self.stutters,
            STUTTER.as_millis()
        );
        *self = Self::new(now);
        Some(line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hitch_names_its_slow_stage_and_lines_are_spaced() {
        let mut hitches = HitchLog::default();
        let start = Instant::now();
        let names = ["world", "render"];
        let stages = [2_000.0, 150_000.0];
        assert_eq!(
            hitches.record(Duration::from_millis(20), start, (&names, &stages)),
            None
        );
        let line = hitches
            .record(Duration::from_millis(160), start, (&names, &stages))
            .expect("a hitch");
        assert_eq!(
            line,
            "hitch: a frame took 160 ms (world 2, render 150, outside drawing 8, in ms)"
        );
        let soon = start + Duration::from_millis(500);
        assert_eq!(
            hitches.record(Duration::from_millis(300), soon, (&names, &stages)),
            None
        );
        let later = start + HITCH_GAP;
        assert!(hitches
            .record(Duration::from_millis(300), later, (&names, &stages))
            .is_some());
    }

    #[test]
    fn a_summary_comes_once_a_minute_and_counts_the_stutters() {
        let start = Instant::now();
        let mut log = StutterLog::new(start);
        assert!(log.record(Duration::from_millis(5), start).is_none());
        assert!(log
            .record(Duration::from_millis(80), start + Duration::from_secs(1))
            .is_none());
        let line = log
            .record(Duration::from_millis(5), start + PERIOD)
            .expect("a minute passed");
        assert!(line.contains("slowest 80 ms"), "{line}");
        assert!(line.contains("1 over 50 ms"), "{line}");
        assert_eq!(log.frames, 0, "the next period starts empty");
    }
}
