// SPDX-License-Identifier: MIT OR Apache-2.0

//! Render scale: the world at a fraction of the screen resolution.
//!
//! On a weak integrated GPU or a software rasterizer (no GPU at all) most of
//! the frame is spent filling pixels (`docs/perf-log.md`, 2026-11-14). With a
//! scale under one the world pass draws into a smaller colour and depth
//! target, which is then stretched over the screen; the first-person hand,
//! the screen overlays and the HUD are drawn after it at the full resolution,
//! so text stays sharp. At `1.0` nothing is allocated and the world is drawn
//! straight to the screen as before.
//!
//! Not a vanilla option (OptiFine has the same idea as "Render Quality").

use crate::renderer::DEPTH_FORMAT;

/// The smallest scale accepted.
pub const MIN_RENDER_SCALE: f32 = 0.25;

/// The size of the scaled target for a `full`-pixel edge.
fn scaled(full: u32, scale: f32) -> u32 {
    ((full as f32 * scale).round() as u32).clamp(1, full.max(1))
}

/// The textures that depend on the window size and the scale.
struct Targets {
    color: wgpu::TextureView,
    depth: wgpu::TextureView,
    group: wgpu::BindGroup,
}

/// The scaled world target and the pipeline that stretches it.
pub(crate) struct RenderScale {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    nearest: wgpu::Sampler,
    linear: wgpu::Sampler,
    format: wgpu::TextureFormat,
    scale: f32,
    smooth: bool,
    size: (u32, u32),
    targets: Option<Targets>,
}

impl RenderScale {
    /// Creates the stage at scale `1.0` (inactive) for a screen in `format`.
    pub(crate) fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        size: (u32, u32),
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("render scale shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("render_scale.wgsl").into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("render scale layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("render scale pipeline layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render scale upscale"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_full"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_upscale"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            // Drawn inside the screen pass, which has a depth attachment; it
            // neither tests nor writes it.
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let sampler = |filter: wgpu::FilterMode| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("render scale sampler"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                mag_filter: filter,
                min_filter: filter,
                ..Default::default()
            })
        };
        Self {
            pipeline,
            layout,
            nearest: sampler(wgpu::FilterMode::Nearest),
            linear: sampler(wgpu::FilterMode::Linear),
            format,
            scale: 1.0,
            smooth: false,
            size,
            targets: None,
        }
    }

    /// Sets the scale (clamped to [`MIN_RENDER_SCALE`]`..=1.0`) and whether
    /// the stretch filters linearly (`smooth`) or keeps square pixels.
    /// Rebuilds the targets only when something changed.
    pub(crate) fn set(&mut self, device: &wgpu::Device, scale: f32, smooth: bool) {
        let scale = if scale.is_finite() {
            scale.clamp(MIN_RENDER_SCALE, 1.0)
        } else {
            1.0
        };
        if scale == self.scale && smooth == self.smooth {
            return;
        }
        self.scale = scale;
        self.smooth = smooth;
        self.rebuild(device);
    }

    /// The current scale.
    pub(crate) fn scale(&self) -> f32 {
        self.scale
    }

    /// Rebuilds the targets for a new screen size.
    pub(crate) fn resize(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        if size != self.size {
            self.size = size;
            self.rebuild(device);
        }
    }

    fn rebuild(&mut self, device: &wgpu::Device) {
        let (width, height) = (
            scaled(self.size.0, self.scale),
            scaled(self.size.1, self.scale),
        );
        if (width, height) == self.size {
            self.targets = None;
            return;
        }
        let texture = |label: &'static str, format: wgpu::TextureFormat| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default())
        };
        let color = texture("render scale colour", self.format);
        let depth = texture("render scale depth", DEPTH_FORMAT);
        let sampler = if self.smooth {
            &self.linear
        } else {
            &self.nearest
        };
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("render scale group"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&color),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });
        self.targets = Some(Targets {
            color,
            depth,
            group,
        });
    }

    /// The colour and depth targets of the world while the scale is under one.
    pub(crate) fn world_targets(&self) -> Option<(&wgpu::TextureView, &wgpu::TextureView)> {
        self.targets.as_ref().map(|t| (&t.color, &t.depth))
    }

    /// Stretches the scaled world over the target of `pass`.
    pub(crate) fn upscale<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        if let Some(targets) = &self.targets {
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &targets.group, &[]);
            pass.draw(0..3, 0..1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_render_scale_shader_is_valid_wgsl() {
        let module = naga::front::wgsl::parse_str(include_str!("render_scale.wgsl"))
            .expect("render_scale.wgsl parses");
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .expect("render_scale.wgsl validates");
    }

    #[test]
    fn the_scaled_target_is_never_empty_nor_larger_than_the_screen() {
        assert_eq!(scaled(1280, 0.5), 640);
        assert_eq!(scaled(1, 0.25), 1);
        assert_eq!(scaled(0, 0.5), 1);
        assert_eq!(scaled(1920, 1.0), 1920);
        assert_eq!(scaled(719, 0.5), 360);
    }
}
