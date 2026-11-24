// SPDX-License-Identifier: MIT OR Apache-2.0

//! The timings of the game thread that the F3 screen shows.

use std::time::{Duration, Instant};

/// The timings of the game thread: tick, network and input times and the TPS.
pub(in crate::game) struct TickMetrics {
    /// Execution time of the previous 20 Hz simulation tick, in microseconds.
    pub(in crate::game) last_tick_micros: u32,
    /// Maximum tick execution time observed, in microseconds.
    pub(in crate::game) max_tick_micros: u32,
    /// Delay of the latest tick from its scheduled instant, in microseconds.
    pub(in crate::game) last_tick_delay_us: u32,
    /// Maximum tick delay observed, in microseconds.
    pub(in crate::game) max_tick_delay_us: u32,
    /// Cumulative catch-up simulation ticks executed.
    pub(in crate::game) catch_up_ticks_total: u64,
    /// Measured simulation TPS.
    pub(in crate::game) real_tps: f32,
    /// Last instant TPS was recalculated.
    pub(in crate::game) last_tps_instant: Instant,
    /// Tick count at last TPS recalculation.
    pub(in crate::game) last_tps_ticks: u64,
    /// Current number of unread events in the network inbox.
    pub(in crate::game) net_queue_len: usize,
    /// Cumulative microseconds spent handling network events since the last tick.
    pub(in crate::game) net_micros_accum: u32,
    /// Microseconds spent handling network events in the previous tick window.
    pub(in crate::game) last_net_micros: u32,
    /// Execution time of the most recent input handling pass, in microseconds.
    pub(in crate::game) last_input_processing_us: u32,
    /// Queuing latency of the most recent input update, in microseconds.
    pub(in crate::game) last_input_queue_latency_us: u32,
    /// The game tick of the last catch-up tick, for the setback lines.
    pub(in crate::game) last_catch_up_tick: Option<u64>,
    /// The game tick at which the last knockback of the player arrived.
    pub(in crate::game) knockback_tick: Option<u64>,
    /// The minute summary of the game thread in the log.
    pub(in crate::game) minute: MinuteHealth,
}

/// How punctual the game thread was over the last minute: late ticks are
/// sent in bursts, which servers read as a timer cheat or as knockback
/// taken a tick late, and answer with a setback.
#[derive(Debug, Clone, Copy)]
pub(in crate::game) struct MinuteHealth {
    since: Instant,
    catch_ups: u32,
    bursts: u32,
    late_max: Duration,
    tick_max: Duration,
    pass_max: Duration,
    split_passes: u32,
}

/// How often the game thread writes its summary.
const MINUTE: Duration = Duration::from_secs(60);

impl MinuteHealth {
    fn new(now: Instant) -> Self {
        Self {
            since: now,
            catch_ups: 0,
            bursts: 0,
            late_max: Duration::ZERO,
            tick_max: Duration::ZERO,
            pass_max: Duration::ZERO,
            split_passes: 0,
        }
    }

    /// Records a run of `ran` ticks, the first `late` after its instant.
    pub(in crate::game) fn ticks(&mut self, ran: u32, late: Duration) {
        self.late_max = self.late_max.max(late);
        if ran > 1 {
            self.bursts += 1;
            self.catch_ups += ran - 1;
        }
    }

    /// Records how long one tick ran.
    pub(in crate::game) fn tick_time(&mut self, elapsed: Duration) {
        self.tick_max = self.tick_max.max(elapsed);
    }

    /// Records a pass over the network events; `split` when it stopped
    /// to let a due tick run.
    pub(in crate::game) fn pass(&mut self, elapsed: Duration, split: bool) {
        self.pass_max = self.pass_max.max(elapsed);
        if split {
            self.split_passes += 1;
        }
    }

    /// The summary line once a minute is over, then a new minute.
    pub(in crate::game) fn line(&mut self, now: Instant) -> Option<String> {
        if now.saturating_duration_since(self.since) < MINUTE {
            return None;
        }
        let ms = |duration: Duration| duration.as_secs_f64() * 1000.0;
        let line = format!(
            "game thread: {} catch-up ticks in {} bursts, latest tick {:.0} ms late, longest tick {:.1} ms, longest packet pass {:.1} ms, {} passes split for a tick",
            self.catch_ups,
            self.bursts,
            ms(self.late_max),
            ms(self.tick_max),
            ms(self.pass_max),
            self.split_passes
        );
        *self = Self::new(now);
        Some(line)
    }
}

impl Default for TickMetrics {
    fn default() -> Self {
        Self {
            last_tick_micros: 0,
            max_tick_micros: 0,
            last_tick_delay_us: 0,
            max_tick_delay_us: 0,
            catch_up_ticks_total: 0,
            real_tps: 20.0,
            last_tps_instant: Instant::now(),
            last_tps_ticks: 0,
            net_queue_len: 0,
            net_micros_accum: 0,
            last_net_micros: 0,
            last_input_processing_us: 0,
            last_input_queue_latency_us: 0,
            last_catch_up_tick: None,
            knockback_tick: None,
            minute: MinuteHealth::new(Instant::now()),
        }
    }
}

impl TickMetrics {
    /// Records how long an input update waited in the queue before the game
    /// thread took it at `now`.
    pub(in crate::game) fn record_input_latency(&mut self, created_at: Instant, now: Instant) {
        self.last_input_queue_latency_us = micros(now.saturating_duration_since(created_at));
    }

    /// Records how long the handling of an input update took.
    pub(in crate::game) fn record_input_processing(&mut self, elapsed: Duration) {
        self.last_input_processing_us = micros(elapsed);
    }

    /// Adds the time one network event took to the current tick window.
    pub(in crate::game) fn record_net_event(&mut self, elapsed: Duration) {
        self.net_micros_accum = self.net_micros_accum.saturating_add(micros(elapsed));
    }

    /// Starts a tick: the network time of the window that ends becomes the
    /// one the F3 screen shows.
    pub(in crate::game) fn start_tick(&mut self) {
        self.last_net_micros = std::mem::take(&mut self.net_micros_accum);
    }

    /// Records how late a tick starts after its scheduled instant, and
    /// whether it is a catch-up tick of a thread that fell behind.
    pub(in crate::game) fn record_tick_delay(&mut self, delay: Duration, catch_up: bool) {
        let delay_us = micros(delay);
        self.max_tick_delay_us = self.max_tick_delay_us.max(delay_us);
        self.last_tick_delay_us = delay_us;
        if catch_up {
            self.catch_up_ticks_total += 1;
        }
    }

    /// Records how long a tick ran.
    pub(in crate::game) fn record_tick_time(&mut self, elapsed: Duration) {
        self.minute.tick_time(elapsed);
        let tick_us = micros(elapsed);
        self.last_tick_micros = tick_us;
        self.max_tick_micros = self.max_tick_micros.max(tick_us);
    }

    /// Measures the ticks per second since the last measure, given the tick
    /// count at `now`. Nothing changes when no time has passed.
    pub(in crate::game) fn update_tps(&mut self, ticks: u64, now: Instant) {
        let elapsed = now.duration_since(self.last_tps_instant).as_secs_f32();
        if elapsed > 0.0 {
            let ticks_delta = ticks.saturating_sub(self.last_tps_ticks);
            self.real_tps = ticks_delta as f32 / elapsed;
            self.last_tps_instant = now;
            self.last_tps_ticks = ticks;
        }
    }
}

/// A duration in whole microseconds, saturated at `u32::MAX` (about 71
/// minutes), which is all the F3 screen needs.
fn micros(duration: Duration) -> u32 {
    u32::try_from(duration.as_micros()).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_minute_line_counts_bursts_and_splits_then_starts_again() {
        let start = Instant::now();
        let mut minute = MinuteHealth::new(start);
        minute.ticks(1, Duration::from_millis(2));
        minute.ticks(3, Duration::from_millis(120));
        minute.tick_time(Duration::from_micros(4_500));
        minute.pass(Duration::from_millis(30), true);
        assert_eq!(minute.line(start + Duration::from_secs(10)), None);
        assert_eq!(
            minute.line(start + MINUTE).as_deref(),
            Some("game thread: 2 catch-up ticks in 1 bursts, latest tick 120 ms late, longest tick 4.5 ms, longest packet pass 30.0 ms, 1 passes split for a tick")
        );
        assert!(minute
            .line(start + MINUTE * 2)
            .unwrap()
            .starts_with("game thread: 0 catch-up"));
    }

    #[test]
    fn a_duration_saturates_at_the_largest_u32() {
        assert_eq!(micros(Duration::from_micros(1_500)), 1_500);
        assert_eq!(micros(Duration::from_secs(10_000)), u32::MAX);
    }

    #[test]
    fn the_tick_delay_keeps_its_maximum_and_counts_catch_ups() {
        let mut metrics = TickMetrics::default();
        metrics.record_tick_delay(Duration::from_micros(900), false);
        metrics.record_tick_delay(Duration::from_micros(300), true);
        assert_eq!(metrics.last_tick_delay_us, 300);
        assert_eq!(metrics.max_tick_delay_us, 900);
        assert_eq!(metrics.catch_up_ticks_total, 1);
    }

    #[test]
    fn the_tick_time_keeps_its_maximum() {
        let mut metrics = TickMetrics::default();
        metrics.record_tick_time(Duration::from_micros(4_000));
        metrics.record_tick_time(Duration::from_micros(1_000));
        assert_eq!(
            (metrics.last_tick_micros, metrics.max_tick_micros),
            (1_000, 4_000)
        );
    }

    #[test]
    fn the_network_time_of_a_window_is_shown_from_the_next_tick() {
        let mut metrics = TickMetrics::default();
        metrics.record_net_event(Duration::from_micros(200));
        metrics.record_net_event(Duration::from_micros(300));
        metrics.start_tick();
        assert_eq!(
            (metrics.last_net_micros, metrics.net_micros_accum),
            (500, 0)
        );
    }

    #[test]
    fn the_input_latency_is_the_wait_in_the_queue() {
        let mut metrics = TickMetrics::default();
        let created = Instant::now();
        metrics.record_input_latency(created, created + Duration::from_micros(750));
        assert_eq!(metrics.last_input_queue_latency_us, 750);
        // A clock read before the creation is no wait at all.
        metrics.record_input_latency(created + Duration::from_secs(1), created);
        assert_eq!(metrics.last_input_queue_latency_us, 0);
    }

    #[test]
    fn forty_ticks_in_two_seconds_are_twenty_per_second() {
        let mut metrics = TickMetrics::default();
        let start = metrics.last_tps_instant;
        metrics.update_tps(40, start + Duration::from_secs(2));
        assert!((metrics.real_tps - 20.0).abs() < 1e-4);
        assert_eq!(metrics.last_tps_ticks, 40);
        // No time has passed: the measure stays.
        metrics.update_tps(80, start + Duration::from_secs(2));
        assert!((metrics.real_tps - 20.0).abs() < 1e-4);
    }
}
