// SPDX-License-Identifier: MIT OR Apache-2.0

//! The three strings of the GPU adapter the F3 screen prints in place of the
//! OpenGL vendor, renderer and version lines of the reference
//! (`GuiOverlayDebug.getDebugInfoRight`).

/// What kind of device draws the frames.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AdapterKind {
    /// A GPU of its own (a graphics card).
    Discrete,
    /// A GPU inside the processor, sharing its memory.
    Integrated,
    /// No GPU: the processor rasterizes (llvmpipe, Lavapipe, WARP, SwiftShader).
    Software,
    /// Unknown or virtual.
    #[default]
    Other,
}

impl AdapterKind {
    fn of(device_type: wgpu::DeviceType) -> Self {
        match device_type {
            wgpu::DeviceType::DiscreteGpu => Self::Discrete,
            wgpu::DeviceType::IntegratedGpu => Self::Integrated,
            wgpu::DeviceType::Cpu => Self::Software,
            wgpu::DeviceType::VirtualGpu | wgpu::DeviceType::Other => Self::Other,
        }
    }
}

/// Name, driver and backend of the adapter the renderer runs on.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AdapterSummary {
    /// The adapter name (`GL_RENDERER` in the reference).
    pub name: String,
    /// The driver name, or the device class when the driver reports none
    /// (`GL_VENDOR` in the reference).
    pub vendor: String,
    /// The graphics API in use, such as `Vulkan` (`GL_VERSION`).
    pub backend: String,
    /// The kind of device, used to choose the first video preset.
    pub kind: AdapterKind,
    /// The raw WGPU backend enum variant, when known.
    pub backend_type: Option<wgpu::Backend>,
}

impl AdapterSummary {
    /// Summarises the adapter the device was created from.
    pub(crate) fn of(adapter: &wgpu::Adapter) -> Self {
        Self::from_info(&adapter.get_info())
    }

    /// Summarises the adapter description `info`.
    pub(crate) fn from_info(info: &wgpu::AdapterInfo) -> Self {
        let vendor = if info.driver.is_empty() {
            format!("{:?}", info.device_type)
        } else {
            info.driver.clone()
        };
        let backend = if info.driver_info.is_empty() {
            format!("{:?}", info.backend)
        } else {
            format!("{:?} {}", info.backend, info.driver_info)
        };
        Self {
            name: info.name.clone(),
            vendor,
            backend,
            kind: AdapterKind::of(info.device_type),
            backend_type: Some(info.backend),
        }
    }
}

/// The graphics adapter the window will use, found without opening a window:
/// the first start picks its video preset from it. Direct3D 12 on Windows,
/// as the window; `None` when no adapter answers.
#[must_use]
pub fn probe_adapter() -> Option<AdapterSummary> {
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    if cfg!(windows) {
        descriptor.backends = wgpu::Backends::DX12;
    }
    let instance = wgpu::Instance::new(descriptor.with_env());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: None,
        apply_limit_buckets: false,
    }))
    .ok()?;
    Some(AdapterSummary::of(&adapter))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(driver: &str, driver_info: &str) -> wgpu::AdapterInfo {
        let mut info = wgpu::AdapterInfo::new(wgpu::DeviceType::DiscreteGpu, wgpu::Backend::Vulkan);
        info.name = "Test GPU".to_owned();
        info.driver = driver.to_owned();
        info.driver_info = driver_info.to_owned();
        info
    }

    #[test]
    fn the_driver_names_the_vendor_when_it_is_known() {
        let summary = AdapterSummary::from_info(&info("radv", "Mesa 25"));
        assert_eq!(summary.name, "Test GPU");
        assert_eq!(summary.vendor, "radv");
        assert_eq!(summary.backend, "Vulkan Mesa 25");
    }

    #[test]
    fn the_device_class_replaces_a_missing_driver() {
        let summary = AdapterSummary::from_info(&info("", ""));
        assert_eq!(summary.vendor, "DiscreteGpu");
        assert_eq!(summary.backend, "Vulkan");
    }
}
