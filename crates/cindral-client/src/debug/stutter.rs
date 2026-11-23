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
