// SPDX-License-Identifier: MIT OR Apache-2.0

//! The screen resolutions the monitor offers, for the Display page.
//!
//! The window reads them from the monitor it opens on (only the event loop
//! can) and stores them here; the options screen, which has no window,
//! lists them. Largest first, each size once whatever its refresh rates.

use std::sync::Mutex;

static MODES: Mutex<Vec<(u32, u32)>> = Mutex::new(Vec::new());

/// Stores the sizes of the monitor's video modes: sorted largest first,
/// duplicates (other refresh rates, bit depths) removed.
pub fn set(sizes: impl IntoIterator<Item = (u32, u32)>) {
    let mut sizes: Vec<(u32, u32)> = sizes
        .into_iter()
        .filter(|&(width, height)| width >= 640 && height >= 480)
        .collect();
    sizes.sort_unstable_by(|a, b| {
        (u64::from(b.0) * u64::from(b.1), b.0).cmp(&(u64::from(a.0) * u64::from(a.1), a.0))
    });
    sizes.dedup();
    if let Ok(mut modes) = MODES.lock() {
        *modes = sizes;
    }
}

/// The resolutions to offer, largest first; empty before the window opened.
#[must_use]
pub fn get() -> Vec<(u32, u32)> {
    MODES.lock().map(|modes| modes.clone()).unwrap_or_default()
}

/// `(width, height)` as the Display option stores it.
#[must_use]
pub fn format(size: (u32, u32)) -> String {
    format!("{}x{}", size.0, size.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sizes_are_listed_once_largest_first_and_tiny_ones_dropped() {
        set([
            (1280, 720),
            (1920, 1080),
            (1280, 720),
            (320, 200),
            (1600, 900),
            (1280, 1024),
        ]);
        assert_eq!(
            get(),
            [(1920, 1080), (1600, 900), (1280, 1024), (1280, 720)]
        );
        assert_eq!(format((1600, 900)), "1600x900");
    }
}
