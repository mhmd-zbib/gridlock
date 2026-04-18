use super::pipeline::{make_pipeline, stencil_read_state};
use super::shader::SHADER;
use super::types::QuadInstance;

use bytemuck::Pod;
use wgpu::util::DeviceExt;

// ---------------------------------------------------------------------------
// Geometry constants
// ---------------------------------------------------------------------------

const VERTICES: &[[f32; 2]] = &[[-1.0, -1.0], [1.0, -1.0], [-1.0, 1.0], [1.0, 1.0]];
const INDICES: &[u16] = &[0, 1, 2, 1, 3, 2];
const MAX_INSTANCES: u64 = 4096;

// ---------------------------------------------------------------------------
// GPU uniform layout (must match shader)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Pod, bytemuck::Zeroable)]
struct Uniforms {
    screen_size: [f32; 2],
    _pad: [f32; 2],
}

// ---------------------------------------------------------------------------
// Renderer
// ---------------------------------------------------------------------------

pub struct Renderer {
    pipeline: wgpu::RenderPipeline,
    pipeline_stencil_equal: wgpu::RenderPipeline,
    pipeline_stencil_not_equal: wgpu::RenderPipeline,
    vertex_buf: wgpu::Buffer,
    index_buf: wgpu::Buffer,
    instance_buf: wgpu::Buffer,
    instance_buf_masked: wgpu::Buffer,
    uniform_buf: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl Renderer {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("quad shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });

        let vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vertices"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("indices"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let instance_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instances"),
            size: MAX_INSTANCES * std::mem::size_of::<QuadInstance>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let instance_buf_masked = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instances (masked)"),
            size: MAX_INSTANCES * std::mem::size_of::<QuadInstance>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bind group"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pipeline layout"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });

        let vertex_buffers = &[
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<[f32; 2]>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x2],
            },
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<QuadInstance>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &wgpu::vertex_attr_array![
                    1 => Float32x2,
                    2 => Float32x2,
                    3 => Float32x4,
                ],
            },
        ];

        let color_target = wgpu::ColorTargetState {
            format,
            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::ALL,
        };

        let pipeline = make_pipeline(
            device,
            &pipeline_layout,
            &shader,
            vertex_buffers,
            &color_target,
            None,
            "fs_plain",
            "quad pipeline (plain)",
        );
        let pipeline_stencil_equal = make_pipeline(
            device,
            &pipeline_layout,
            &shader,
            vertex_buffers,
            &color_target,
            Some(stencil_read_state(wgpu::CompareFunction::Equal)),
            "fs_plain",
            "quad pipeline (stencil == 1)",
        );
        let pipeline_stencil_not_equal = make_pipeline(
            device,
            &pipeline_layout,
            &shader,
            vertex_buffers,
            &color_target,
            Some(stencil_read_state(wgpu::CompareFunction::NotEqual)),
            "fs_plain",
            "quad pipeline (stencil != 1)",
        );

        Self {
            pipeline,
            pipeline_stencil_equal,
            pipeline_stencil_not_equal,
            vertex_buf,
            index_buf,
            instance_buf,
            instance_buf_masked,
            uniform_buf,
            bind_group,
        }
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    fn write_uniforms(&self, queue: &wgpu::Queue, screen_w: f32, screen_h: f32) {
        let u = Uniforms {
            screen_size: [screen_w, screen_h],
            _pad: [0.0; 2],
        };
        queue.write_buffer(&self.uniform_buf, 0, bytemuck::bytes_of(&u));
    }

    fn draw_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        pipeline: &wgpu::RenderPipeline,
        instance_buf: &wgpu::Buffer,
        load: wgpu::LoadOp<wgpu::Color>,
        instances: &[QuadInstance],
    ) {
        if instances.is_empty() {
            return;
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("quad pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buf.slice(..));
        pass.set_vertex_buffer(1, instance_buf.slice(..));
        pass.set_index_buffer(self.index_buf.slice(..), wgpu::IndexFormat::Uint16);
        pass.draw_indexed(0..6, 0, 0..instances.len() as u32);
    }

    fn draw_pass_stenciled(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        stencil_view: &wgpu::TextureView,
        pipeline: &wgpu::RenderPipeline,
        instance_buf: &wgpu::Buffer,
        instances: &[QuadInstance],
    ) {
        if instances.is_empty() {
            return;
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("quad pass (stenciled)"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: stencil_view,
                depth_ops: None,
                stencil_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
            }),
            ..Default::default()
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buf.slice(..));
        pass.set_vertex_buffer(1, instance_buf.slice(..));
        pass.set_index_buffer(self.index_buf.slice(..), wgpu::IndexFormat::Uint16);
        pass.set_stencil_reference(1);
        pass.draw_indexed(0..6, 0, 0..instances.len() as u32);
    }

    // ── Public draw calls ─────────────────────────────────────────────────────

    pub fn draw(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        queue: &wgpu::Queue,
        screen_w: f32,
        screen_h: f32,
        instances: &[QuadInstance],
    ) {
        self.write_uniforms(queue, screen_w, screen_h);
        if !instances.is_empty() {
            queue.write_buffer(&self.instance_buf, 0, bytemuck::cast_slice(instances));
        }
        self.draw_pass(
            encoder,
            view,
            &self.pipeline,
            &self.instance_buf,
            wgpu::LoadOp::Clear(wgpu::Color::BLACK),
            instances,
        );
    }

    pub fn draw_load(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        queue: &wgpu::Queue,
        screen_w: f32,
        screen_h: f32,
        instances: &[QuadInstance],
    ) {
        self.write_uniforms(queue, screen_w, screen_h);
        if !instances.is_empty() {
            queue.write_buffer(&self.instance_buf, 0, bytemuck::cast_slice(instances));
        }
        self.draw_pass(
            encoder,
            view,
            &self.pipeline,
            &self.instance_buf,
            wgpu::LoadOp::Load,
            instances,
        );
    }

    pub fn draw_masked(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        stencil_view: &wgpu::TextureView,
        queue: &wgpu::Queue,
        screen_w: f32,
        screen_h: f32,
        instances: &[QuadInstance],
    ) {
        self.write_uniforms(queue, screen_w, screen_h);
        if !instances.is_empty() {
            queue.write_buffer(
                &self.instance_buf_masked,
                0,
                bytemuck::cast_slice(instances),
            );
        }
        self.draw_pass_stenciled(
            encoder,
            view,
            stencil_view,
            &self.pipeline_stencil_equal,
            &self.instance_buf_masked,
            instances,
        );
    }

    pub fn draw_dim_overlay(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        stencil_view: &wgpu::TextureView,
        queue: &wgpu::Queue,
        screen_w: f32,
        screen_h: f32,
        outside_dim: f32,
    ) {
        let alpha = outside_dim.clamp(0.0, 1.0);
        if alpha <= 0.0 {
            return;
        }
        self.write_uniforms(queue, screen_w, screen_h);
        let overlay = [QuadInstance {
            center: [screen_w * 0.5, screen_h * 0.5],
            half_size: [screen_w * 0.5, screen_h * 0.5],
            color: [0.0, 0.0, 0.0, alpha],
        }];
        queue.write_buffer(&self.instance_buf, 0, bytemuck::cast_slice(&overlay));
        self.draw_pass_stenciled(
            encoder,
            view,
            stencil_view,
            &self.pipeline_stencil_not_equal,
            &self.instance_buf,
            &overlay,
        );
    }
}
