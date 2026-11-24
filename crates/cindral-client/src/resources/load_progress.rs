// SPDX-License-Identifier: MIT OR Apache-2.0

//! How far the start has come, for the loading screen.
//!
//! The asset thread moves through the steps below and stores each one here;
//! the window reads it every frame. The bar shows the real step, so a slow
//! disk shows a slow bar instead of an animation that loops forever.

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// One step of the start, in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Step {
    /// Opening the client jar and the resource packs.
    Packs,
    /// Building the block models and the texture atlas.
    Models,
    /// Reading the sky, entity and item pictures.
    Files,
    /// Reading the font.
    Font,
    /// Sending the font and block textures to the graphics card (on the
    /// window thread).
    Upload,
    /// Sending the rest and building the world view (on the window thread).
    World,
    /// Done: the title screen is next.
    Done,
}

impl Step {
    const ALL: [Self; 7] = [
        Self::Packs,
        Self::Models,
        Self::Files,
        Self::Font,
        Self::Upload,
        Self::World,
        Self::Done,
    ];

    /// The share of the bar filled once this step has started. The steps
    /// are weighted by how long they took on a Pentium G5400.
    #[must_use]
    pub fn fraction(self) -> f32 {
        match self {
            Self::Packs => 0.04,
            Self::Models => 0.18,
            Self::Files => 0.62,
            Self::Font => 0.70,
            Self::Upload => 0.74,
            Self::World => 0.9,
            Self::Done => 1.0,
        }
    }

    /// What the screen says during the step.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Packs => "Opening resource packs",
            Self::Models => "Building block models",
            Self::Files => "Reading textures",
            Self::Font => "Loading the font",
            Self::Upload => "Uploading block textures",
            Self::World => "Preparing the world",
            Self::Done => "Ready",
        }
    }
}

/// The current step, shared between the asset thread and the window.
#[derive(Debug, Clone, Default)]
pub struct LoadProgress(Arc<AtomicU8>);

impl LoadProgress {
    /// Records that `step` has started. Steps only move forward.
    pub fn set(&self, step: Step) {
        self.0.fetch_max(step as u8, Ordering::Relaxed);
    }

    /// The step in progress.
    #[must_use]
    pub fn step(&self) -> Step {
        let index = usize::from(self.0.load(Ordering::Relaxed));
        Step::ALL[index.min(Step::ALL.len() - 1)]
    }
}

/// How long the fade from the loading screen to the title screen takes.
pub const FADE: Duration = Duration::from_millis(300);
/// The loading screen stays at least this long once shown, so a fast start
/// does not flash it for a single frame.
pub const MINIMUM: Duration = Duration::from_millis(600);

/// What the window keeps about the loading screen.
#[derive(Debug)]
pub struct Splash {
    /// The step the asset thread reached.
    pub progress: LoadProgress,
    /// When the loading screen first reached the screen.
    pub shown_at: Option<Instant>,
    /// The filled share of the bar drawn last frame; it eases towards the
    /// step's share instead of jumping.
    pub drawn: f32,
    /// The last frame that moved the bar.
    last: Option<Instant>,
    /// When the loading screen started fading out.
    pub fade_from: Option<Instant>,
}

impl Splash {
    /// A splash at the first step.
    #[must_use]
    pub fn new(progress: LoadProgress) -> Self {
        Self {
            progress,
            shown_at: None,
            drawn: 0.0,
            last: None,
            fade_from: None,
        }
    }

    /// Moves the bar towards the current step and returns its filled share.
    pub fn advance(&mut self, now: Instant) -> f32 {
        let target = self.progress.step().fraction();
        let seconds = self.last.map_or(0.0, |last| {
            now.saturating_duration_since(last).as_secs_f32()
        });
        self.last = Some(now);
        // Most of the way in about a quarter of a second; never past the
        // step, never backwards.
        let follow = 1.0 - (-seconds * 9.0).exp();
        self.drawn = (self.drawn + (target - self.drawn) * follow).clamp(self.drawn, target);
        if target - self.drawn < 0.002 {
            self.drawn = target;
        }
        self.drawn
    }

    /// Whether the loading screen has been seen long enough to leave it.
    #[must_use]
    pub fn may_leave(&self, now: Instant) -> bool {
        self.shown_at
            .is_some_and(|shown| now.saturating_duration_since(shown) >= MINIMUM)
    }

    /// The opacity of the fading loading screen over the next screen, `None`
    /// once it is gone.
    #[must_use]
    pub fn fade(&self, now: Instant) -> Option<f32> {
        let from = self.fade_from?;
        let gone = now.saturating_duration_since(from).as_secs_f32() / FADE.as_secs_f32();
        (gone < 1.0).then_some(1.0 - gone * gone)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_steps_only_move_forward_and_fill_the_bar_in_order() {
        let progress = LoadProgress::default();
        assert_eq!(progress.step(), Step::Packs);
        progress.set(Step::Files);
        progress.set(Step::Models);
        assert_eq!(progress.step(), Step::Files);
        let shares: Vec<f32> = Step::ALL.iter().map(|step| step.fraction()).collect();
        assert!(shares.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(Step::Done.fraction(), 1.0);
    }

    #[test]
    fn the_bar_eases_to_the_step_without_passing_it() {
        let progress = LoadProgress::default();
        progress.set(Step::Font);
        let mut splash = Splash::new(progress);
        let start = Instant::now();
        assert_eq!(splash.advance(start), 0.0, "the first frame starts empty");
        let quarter = splash.advance(start + Duration::from_millis(100));
        assert!(quarter > 0.3 && quarter < Step::Font.fraction());
        let later = splash.advance(start + Duration::from_secs(2));
        assert_eq!(later, Step::Font.fraction());
    }

    #[test]
    fn the_fade_runs_out_and_the_screen_waits_its_minimum() {
        let mut splash = Splash::new(LoadProgress::default());
        let start = Instant::now();
        assert!(!splash.may_leave(start), "never shown");
        splash.shown_at = Some(start);
        assert!(!splash.may_leave(start + Duration::from_millis(100)));
        assert!(splash.may_leave(start + MINIMUM));
        assert_eq!(splash.fade(start), None);
        splash.fade_from = Some(start);
        assert_eq!(splash.fade(start), Some(1.0));
        assert!(splash
            .fade(start + FADE / 2)
            .is_some_and(|alpha| alpha > 0.5));
        assert_eq!(splash.fade(start + FADE), None);
    }
}
