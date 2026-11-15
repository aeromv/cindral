// SPDX-License-Identifier: MIT OR Apache-2.0

//! Name tags drawn in the world pass with the scene's depth buffer.
//!
//! Vanilla draws a standing player's tag twice (MCP `Render.renderLivingLabel`):
//! the box and a faint copy of the text with the depth test off, so they show
//! through terrain, then the text again at full strength with the depth test
//! on. A crouching player's tag (`RendererLivingEntity.renderName`) is only the
//! box and the faint text, with the depth test on. The UI overlay has no depth
//! test, so the tags come here instead: each one is a flat quad at the depth
//! of its anchor, which is what a billboard is.
//!
//! Every tag of the frame is one draw with one pipeline: the depth is a vertex
//! attribute, the anchor's for the tested parts and 0 for the parts drawn
//! regardless (the near plane passes the less-equal test everywhere, as the
//! overlay pipeline's `Always` does, with the same blending and shader). The
//! vertices keep the order of the tags, so the blending is unchanged; two
//! draws and two pipeline switches per tag cost an integrated GPU more than
//! the tags themselves (a hub with 115 tags in view).

use std::ops::Range;

use cindral_ui::{DrawList, FontAtlas};

use crate::batch_buffer::GrowBuffer;
use crate::overlay::OverlayVertex;
use crate::renderer::{DEPTH_FORMAT, OVERLAY_ATTRIBUTES};
use crate::ui_pass::UiPass;

/// One tag: what is drawn regardless of the scene, what is drawn only where
/// nothing hides it, and the depth (`0..1`) of the anchor. The lists are in
/// window pixels and hold rectangles and text only.
#[derive(Debug, Clone, Default)]
pub struct Nametag {
    /// Drawn without a depth test.
    pub always: DrawList,
    /// Drawn where the tag is not behind the scene.
    pub tested: DrawList,
    /// Normalised device depth of the anchor, as the world pass writes it.
    pub depth: f32,
}

/// The depth-tested pipeline and the buffers of the tags of the frame.
pub(crate) struct NametagRenderer {
    pipeline: wgpu::RenderPipeline,
    scratch: UiPass,
    vertices: GrowBuffer,
    /// One depth per vertex.
    depths: GrowBuffer,
    /// The vertices to draw this frame.
    count: Range<u32>,
    /// What the buffers hold this frame: the vertices of every tag and the
    /// depth of each vertex, rebuilt in place so a steady frame allocates nothing.
    vertex_list: Vec<OverlayVertex>,
    depth_list: Vec<f32>,
    /// Whether the frame has tags to draw.
    active: bool,
}

impl NametagRenderer {
    pub(crate) fn new(
        device: &wgpu::Device,
        shader: &wgpu::ShaderModule,
        layout: &wgpu::BindGroupLayout,
        format: wgpu::TextureFormat,
    ) -> Self {
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("nametag pipeline layout"),
            bind_group_layouts: &[Some(layout)],
            immediate_size: 0,
        });
        let depth_attribute = [wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32,
            offset: 0,
            shader_location: 3,
        }];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("nametag pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("vs_tag"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[
                    Some(wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<OverlayVertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &OVERLAY_ATTRIBUTES,
                    }),
                    Some(wgpu::VertexBufferLayout {
                        array_stride: 4,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &depth_attribute,
                    }),
                ],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        Self {
            pipeline,
            scratch: UiPass::new(1, 1),
            vertices: GrowBuffer::default(),
            depths: GrowBuffer::default(),
            count: 0..0,
            vertex_list: Vec::new(),
            depth_list: Vec::new(),
            active: false,
        }
    }

    /// Replaces the tags of the frame; `size` is the window in pixels.
    ///
    /// The buffers persist and only grow, and the lists that fill them are
    /// reused (F2-5b); a frame with no tag writes nothing.
    pub(crate) fn set(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        tags: &[Nametag],
        size: (u32, u32),
        atlas: &FontAtlas,
    ) {
        self.count = 0..0;
        self.vertex_list.clear();
        self.depth_list.clear();
        self.active = false;
        if tags.is_empty() {
            return;
        }
        self.scratch.resize(size.0, size.1);
        for tag in tags {
            let mut append = |list: &DrawList, depth: f32| {
                self.scratch.rebuild(list, atlas);
                self.vertex_list.extend_from_slice(self.scratch.vertices());
                self.depth_list.resize(self.vertex_list.len(), depth);
            };
            // The near plane: drawn whatever the scene holds.
            append(&tag.always, 0.0);
            append(&tag.tested, tag.depth);
        }
        // Tags clipped away entirely leave nothing to bind (an empty buffer
        // slice is invalid).
        if self.vertex_list.is_empty() {
            return;
        }
        self.count = 0..self.vertex_list.len() as u32;
        self.vertices.write(
            device,
            queue,
            "nametag vertices",
            bytemuck::cast_slice(&self.vertex_list),
        );
        self.depths.write(
            device,
            queue,
            "nametag depths",
            bytemuck::cast_slice(&self.depth_list),
        );
        self.active = true;
    }

    /// Draws the tags: `always` with the overlay pipeline, `tested` with the
    /// depth-tested one, both over the UI font atlas.
    pub(crate) fn draw(&self, pass: &mut wgpu::RenderPass<'_>, font_bind_group: &wgpu::BindGroup) {
        let (true, Some(vertices), Some(depths)) =
            (self.active, self.vertices.get(), self.depths.get())
        else {
            return;
        };
        pass.set_bind_group(0, font_bind_group, &[]);
        pass.set_vertex_buffer(0, vertices.slice(..));
        pass.set_vertex_buffer(1, depths.slice(..));
        pass.set_pipeline(&self.pipeline);
        pass.draw(self.count.clone(), 0..1);
    }
}
