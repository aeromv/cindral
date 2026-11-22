// SPDX-License-Identifier: MIT OR Apache-2.0

//! What the first start knows about the machine, to pick its video preset
//! (`VideoPreset::for_machine`): the GPU, the CPU threads and the memory.

use super::VideoPreset;

/// Picks the preset of this machine and says why, for the log. Costs the
/// time to ask the driver for an adapter (tens of milliseconds), so only the
/// first start calls it.
#[must_use]
pub fn tuned_preset() -> VideoPreset {
    let adapter = cindral_render::probe_adapter();
    let kind = adapter
        .as_ref()
        .map(|adapter| adapter.kind)
        .unwrap_or_default();
    let threads = std::thread::available_parallelism().map_or(4, std::num::NonZero::get);
    let ram_gb = total_ram_gb();
    let preset = VideoPreset::for_machine(kind, threads, ram_gb);
    tracing::info!(
        gpu = adapter
            .as_ref()
            .map_or("unknown", |adapter| adapter.name.as_str()),
        ?kind,
        threads,
        ?ram_gb,
        ?preset,
        "first start: video settings tuned to this machine"
    );
    preset
}

/// The installed memory, in whole gigabytes (rounded up, as a 7.9 GB
/// report means an 8 GB machine).
#[cfg(windows)]
#[allow(unsafe_code)]
fn total_ram_gb() -> Option<u32> {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    let mut status = MEMORYSTATUSEX {
        dwLength: u32::try_from(std::mem::size_of::<MEMORYSTATUSEX>()).ok()?,
        ..Default::default()
    };
    // SAFETY: `status` is a valid MEMORYSTATUSEX with its length set, as the
    // call requires.
    let ok = unsafe { GlobalMemoryStatusEx(&raw mut status) };
    if ok == 0 {
        return None;
    }
    let gb = status.ullTotalPhys.div_ceil(1 << 30);
    u32::try_from(gb).ok()
}

#[cfg(not(windows))]
fn total_ram_gb() -> Option<u32> {
    None
}

#[cfg(test)]
mod tests {
    use cindral_render::AdapterKind;

    use super::*;

    #[test]
    fn a_weak_integrated_pc_gets_the_pvp_preset() {
        // Pentium G5400 (4 threads) with UHD 610.
        assert_eq!(
            VideoPreset::for_machine(AdapterKind::Integrated, 4, Some(8)),
            VideoPreset::Pvp
        );
        assert_eq!(
            VideoPreset::for_machine(AdapterKind::Integrated, 12, Some(16)),
            VideoPreset::Medium
        );
    }

    #[test]
    fn a_graphics_card_keeps_the_full_look_unless_the_rest_is_weak() {
        assert_eq!(
            VideoPreset::for_machine(AdapterKind::Discrete, 12, Some(16)),
            VideoPreset::High
        );
        assert_eq!(
            VideoPreset::for_machine(AdapterKind::Discrete, 4, Some(16)),
            VideoPreset::Medium
        );
        assert_eq!(
            VideoPreset::for_machine(AdapterKind::Software, 16, None),
            VideoPreset::Low
        );
    }
}
