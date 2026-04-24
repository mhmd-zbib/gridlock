use std::cmp::Ordering;

use bytemuck::Pod;
use wgpu::util::DeviceExt;

use crate::asset::TextureCache;
use crate::render::light::LightingUniform;

use super::shader::SHADER;
use super::types::{SpriteCommand, SpriteInstance};

const VERTICES: &[[f32; 2]] = &[[-1.0, -1.0], [1.0, -1.0], [-1.0, 1.0], [1.0, 1.0]];
const INDICES: &[u16] = &[0, 1, 2, 1, 3, 2];
const MAX_SPRITES: u64 = 4096;

#[repr(C)]
#[derive(Copy, Clone, Pod, bytemuck::Zeroable)]
struct Uniforms {
    screen_size: [f32; 2],
    _pad: [f32; 2],
}

/// Textured-quad renderer with per-fragment point-light shading.
///
/// Call [`update_lighting`] once per frame before the first [`draw`] to upload
/// the current light set.  Lighting persists across multiple `draw` calls
/// within the same frame (floor sprites, prop sprites, etc.).
pub struct SpriteRenderer {
    pipeline: wgpu::RenderPipeline,
    vertex_buf: wgpu::Buffer,
    index_buf: wgpu::Buffer,
    instance_buf: wgpu::Buffer,
    uniform_buf: wgpu::Buffer,
    light_buf: wgpu::Buffer,
    uniform_bg: wgpu::BindGroup,
}

impl SpriteRenderer {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        texture_bgl: &wgpu::BindGroupLayout,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sprite shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });

        let vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sprite vertices"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sprite indices"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        let instance_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sprite instances"),
            size: MAX_SPRITES * std::mem::size_of::<SpriteInstance>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sprite uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let light_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sprite lighting"),
            size: std::mem::size_of::<LightingUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bgl_uniform = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sprite uniform bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let uniform_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sprite uniform bg"),
            layout: &bgl_uniform,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: light_buf.as_entire_binding(),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sprite pipeline layout"),
            bind_group_layouts: &[Some(&bgl_uniform), Some(texture_bgl)],
            immediate_size: 0,
        });

        let vertex_buffers = &[
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<[f32; 2]>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x2],
            },
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<SpriteInstance>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &wgpu::vertex_attr_array![
                    1 => Float32x2, // center
                    2 => Float32x2, // half_size
                    3 => Float32,   // rotation
                    4 => Float32x2, // uv_min
                    5 => Float32x2, // uv_max
                    6 => Float32x4, // tint
                ],
            },
        ];

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sprite pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: vertex_buffers,
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
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
            vertex_buf,
            index_buf,
            instance_buf,
            uniform_buf,
            light_buf,
            uniform_bg,
        }
    }

    /// Upload lighting data for the current frame.
    ///
    /// Call once per frame before the first [`draw`].  The data persists in the
    /// uniform buffer and is reused by all subsequent draw calls this frame.
    pub fn update_lighting(&self, queue: &wgpu::Queue, lighting: &LightingUniform) {
        queue.write_buffer(&self.light_buf, 0, bytemuck::bytes_of(lighting));
    }

    /// Draw all `commands` on top of whatever is already in `view`.
    ///
    /// Sort order: ascending z (painter's algorithm), with ties broken by
    /// handle so same-texture sprites are contiguous — one draw call per batch.
    pub fn draw(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        queue: &wgpu::Queue,
        screen_w: f32,
        screen_h: f32,
        commands: &[SpriteCommand],
        cache: &TextureCache,
    ) {
        if commands.is_empty() {
            return;
        }

        // ── Step 1: sort by (z, handle) ──────────────────────────────────────
        let mut sorted: Vec<&SpriteCommand> = commands.iter().collect();
        sorted.sort_unstable_by(|a, b| {
            a.z.partial_cmp(&b.z)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.handle.0.cmp(&b.handle.0))
        });

        // ── Step 2: build instance buffer in sorted order ────────────────────
        let instances: Vec<SpriteInstance> = sorted
            .iter()
            .map(|cmd| SpriteInstance {
                center: cmd.pos,
                half_size: cmd.half_size,
                rotation: cmd.rotation,
                uv_min: [cmd.uv_rect[0], cmd.uv_rect[1]],
                uv_max: [cmd.uv_rect[2], cmd.uv_rect[3]],
                tint: cmd.tint,
            })
            .collect();

        queue.write_buffer(&self.instance_buf, 0, bytemuck::cast_slice(&instances));
        queue.write_buffer(
            &self.uniform_buf,
            0,
            bytemuck::bytes_of(&Uniforms {
                screen_size: [screen_w, screen_h],
                _pad: [0.0; 2],
            }),
        );

        // ── Step 3: one render pass, rebind texture per batch ────────────────
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("sprite pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.uniform_bg, &[]);
        pass.set_vertex_buffer(0, self.vertex_buf.slice(..));
        pass.set_vertex_buffer(1, self.instance_buf.slice(..));
        pass.set_index_buffer(self.index_buf.slice(..), wgpu::IndexFormat::Uint16);

        let mut batch_start = 0usize;
        while batch_start < sorted.len() {
            let handle = sorted[batch_start].handle;

            let Some(bg) = cache.bind_group(handle) else {
                // Texture not ready — skip this sprite.
                batch_start += 1;
                continue;
            };

            // Find the end of this contiguous same-handle run.
            let batch_end = sorted[batch_start..]
                .iter()
                .position(|cmd| cmd.handle != handle)
                .map(|rel| batch_start + rel)
                .unwrap_or(sorted.len());

            pass.set_bind_group(1, bg, &[]);
            pass.draw_indexed(0..6, 0, batch_start as u32..batch_end as u32);

            batch_start = batch_end;
        }
    }
}
