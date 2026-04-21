use super::pipeline::{make_pipeline, stencil_read_state};
use super::shader::SHADER;
use super::types::{GradientQuadInstance, QuadInstance, ShadedQuadInstance};

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
    gradient_pipeline: wgpu::RenderPipeline,
    gradient_pipeline_stencil_equal: wgpu::RenderPipeline,
    gradient_pipeline_stencil_not_equal: wgpu::RenderPipeline,
    shaded_pipeline: wgpu::RenderPipeline,
    shaded_pipeline_stencil_equal: wgpu::RenderPipeline,
    shaded_pipeline_stencil_not_equal: wgpu::RenderPipeline,
    vertex_buf: wgpu::Buffer,
    index_buf: wgpu::Buffer,
    instance_buf: wgpu::Buffer,
    instance_buf_masked: wgpu::Buffer,
    gradient_instance_buf: wgpu::Buffer,
    gradient_instance_buf_masked: wgpu::Buffer,
    shaded_instance_buf: wgpu::Buffer,
    shaded_instance_buf_masked: wgpu::Buffer,
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

        let gradient_instance_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gradient instances"),
            size: MAX_INSTANCES * std::mem::size_of::<GradientQuadInstance>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let gradient_instance_buf_masked = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gradient instances (masked)"),
            size: MAX_INSTANCES * std::mem::size_of::<GradientQuadInstance>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let shaded_instance_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shaded instances"),
            size: MAX_INSTANCES * std::mem::size_of::<ShadedQuadInstance>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let shaded_instance_buf_masked = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shaded instances (masked)"),
            size: MAX_INSTANCES * std::mem::size_of::<ShadedQuadInstance>() as u64,
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

        let solid_vertex_buffers = &[
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
        let gradient_vertex_buffers = &[
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<[f32; 2]>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x2],
            },
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<GradientQuadInstance>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &wgpu::vertex_attr_array![
                    1 => Float32x2,
                    2 => Float32x2,
                    3 => Float32x4,
                    4 => Float32x4,
                ],
            },
        ];
        let shaded_vertex_buffers = &[
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<[f32; 2]>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x2],
            },
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<ShadedQuadInstance>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &wgpu::vertex_attr_array![
                    1 => Float32x2,
                    2 => Float32x2,
                    3 => Float32x4,
                    4 => Float32x4,
                    5 => Float32x4,
                    6 => Float32x4,
                    7 => Float32x4,
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
            solid_vertex_buffers,
            &color_target,
            None,
            "vs_main",
            "fs_plain",
            "quad pipeline (plain)",
        );
        let pipeline_stencil_equal = make_pipeline(
            device,
            &pipeline_layout,
            &shader,
            solid_vertex_buffers,
            &color_target,
            Some(stencil_read_state(wgpu::CompareFunction::Equal)),
            "vs_main",
            "fs_plain",
            "quad pipeline (stencil == 1)",
        );
        let pipeline_stencil_not_equal = make_pipeline(
            device,
            &pipeline_layout,
            &shader,
            solid_vertex_buffers,
            &color_target,
            Some(stencil_read_state(wgpu::CompareFunction::NotEqual)),
            "vs_main",
            "fs_plain",
            "quad pipeline (stencil != 1)",
        );
        let gradient_pipeline = make_pipeline(
            device,
            &pipeline_layout,
            &shader,
            gradient_vertex_buffers,
            &color_target,
            None,
            "vs_gradient",
            "fs_gradient",
            "quad gradient pipeline (plain)",
        );
        let gradient_pipeline_stencil_equal = make_pipeline(
            device,
            &pipeline_layout,
            &shader,
            gradient_vertex_buffers,
            &color_target,
            Some(stencil_read_state(wgpu::CompareFunction::Equal)),
            "vs_gradient",
            "fs_gradient",
            "quad gradient pipeline (stencil == 1)",
        );
        let gradient_pipeline_stencil_not_equal = make_pipeline(
            device,
            &pipeline_layout,
            &shader,
            gradient_vertex_buffers,
            &color_target,
            Some(stencil_read_state(wgpu::CompareFunction::NotEqual)),
            "vs_gradient",
            "fs_gradient",
            "quad gradient pipeline (stencil != 1)",
        );
        let shaded_pipeline = make_pipeline(
            device,
            &pipeline_layout,
            &shader,
            shaded_vertex_buffers,
            &color_target,
            None,
            "vs_shaded",
            "fs_shaded",
            "quad shaded pipeline (plain)",
        );
        let shaded_pipeline_stencil_equal = make_pipeline(
            device,
            &pipeline_layout,
            &shader,
            shaded_vertex_buffers,
            &color_target,
            Some(stencil_read_state(wgpu::CompareFunction::Equal)),
            "vs_shaded",
            "fs_shaded",
            "quad shaded pipeline (stencil == 1)",
        );
        let shaded_pipeline_stencil_not_equal = make_pipeline(
            device,
            &pipeline_layout,
            &shader,
            shaded_vertex_buffers,
            &color_target,
            Some(stencil_read_state(wgpu::CompareFunction::NotEqual)),
            "vs_shaded",
            "fs_shaded",
            "quad shaded pipeline (stencil != 1)",
        );

        Self {
            pipeline,
            pipeline_stencil_equal,
            pipeline_stencil_not_equal,
            gradient_pipeline,
            gradient_pipeline_stencil_equal,
            gradient_pipeline_stencil_not_equal,
            shaded_pipeline,
            shaded_pipeline_stencil_equal,
            shaded_pipeline_stencil_not_equal,
            vertex_buf,
            index_buf,
            instance_buf,
            instance_buf_masked,
            gradient_instance_buf,
            gradient_instance_buf_masked,
            shaded_instance_buf,
            shaded_instance_buf_masked,
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
        instance_count: u32,
        force_pass: bool,
    ) {
        if instance_count == 0 && !force_pass {
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
        if instance_count == 0 {
            return;
        }
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buf.slice(..));
        pass.set_vertex_buffer(1, instance_buf.slice(..));
        pass.set_index_buffer(self.index_buf.slice(..), wgpu::IndexFormat::Uint16);
        pass.draw_indexed(0..6, 0, 0..instance_count);
    }

    fn draw_pass_stenciled(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        stencil_view: &wgpu::TextureView,
        pipeline: &wgpu::RenderPipeline,
        instance_buf: &wgpu::Buffer,
        instance_count: u32,
    ) {
        if instance_count == 0 {
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
        pass.draw_indexed(0..6, 0, 0..instance_count);
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
        let instance_count = instances.len() as u32;
        self.draw_pass(
            encoder,
            view,
            &self.pipeline,
            &self.instance_buf,
            wgpu::LoadOp::Clear(wgpu::Color::BLACK),
            instance_count,
            true,
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
        let instance_count = instances.len() as u32;
        self.draw_pass(
            encoder,
            view,
            &self.pipeline,
            &self.instance_buf,
            wgpu::LoadOp::Load,
            instance_count,
            false,
        );
    }

    pub fn draw_inside_cone(
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
            queue.write_buffer(&self.instance_buf, 0, bytemuck::cast_slice(instances));
        }
        let instance_count = instances.len() as u32;
        self.draw_pass_stenciled(
            encoder,
            view,
            stencil_view,
            &self.pipeline_stencil_equal,
            &self.instance_buf,
            instance_count,
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
        let instance_count = instances.len() as u32;
        self.draw_pass_stenciled(
            encoder,
            view,
            stencil_view,
            &self.pipeline_stencil_equal,
            &self.instance_buf_masked,
            instance_count,
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
            overlay.len() as u32,
        );
    }

    pub fn draw_gradient(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        queue: &wgpu::Queue,
        screen_w: f32,
        screen_h: f32,
        instances: &[GradientQuadInstance],
    ) {
        self.write_uniforms(queue, screen_w, screen_h);
        if !instances.is_empty() {
            queue.write_buffer(
                &self.gradient_instance_buf,
                0,
                bytemuck::cast_slice(instances),
            );
        }
        let instance_count = instances.len() as u32;
        self.draw_pass(
            encoder,
            view,
            &self.gradient_pipeline,
            &self.gradient_instance_buf,
            wgpu::LoadOp::Clear(wgpu::Color::BLACK),
            instance_count,
            true,
        );
    }

    pub fn draw_gradient_load(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        queue: &wgpu::Queue,
        screen_w: f32,
        screen_h: f32,
        instances: &[GradientQuadInstance],
    ) {
        self.write_uniforms(queue, screen_w, screen_h);
        if !instances.is_empty() {
            queue.write_buffer(
                &self.gradient_instance_buf,
                0,
                bytemuck::cast_slice(instances),
            );
        }
        let instance_count = instances.len() as u32;
        self.draw_pass(
            encoder,
            view,
            &self.gradient_pipeline,
            &self.gradient_instance_buf,
            wgpu::LoadOp::Load,
            instance_count,
            false,
        );
    }

    pub fn draw_gradient_masked(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        stencil_view: &wgpu::TextureView,
        queue: &wgpu::Queue,
        screen_w: f32,
        screen_h: f32,
        instances: &[GradientQuadInstance],
    ) {
        self.write_uniforms(queue, screen_w, screen_h);
        if !instances.is_empty() {
            queue.write_buffer(
                &self.gradient_instance_buf_masked,
                0,
                bytemuck::cast_slice(instances),
            );
        }
        let instance_count = instances.len() as u32;
        self.draw_pass_stenciled(
            encoder,
            view,
            stencil_view,
            &self.gradient_pipeline_stencil_equal,
            &self.gradient_instance_buf_masked,
            instance_count,
        );
    }

    pub fn draw_gradient_outside_cone(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        stencil_view: &wgpu::TextureView,
        queue: &wgpu::Queue,
        screen_w: f32,
        screen_h: f32,
        instances: &[GradientQuadInstance],
    ) {
        self.write_uniforms(queue, screen_w, screen_h);
        if !instances.is_empty() {
            queue.write_buffer(
                &self.gradient_instance_buf_masked,
                0,
                bytemuck::cast_slice(instances),
            );
        }
        let instance_count = instances.len() as u32;
        self.draw_pass_stenciled(
            encoder,
            view,
            stencil_view,
            &self.gradient_pipeline_stencil_not_equal,
            &self.gradient_instance_buf_masked,
            instance_count,
        );
    }

    pub fn draw_shaded(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        queue: &wgpu::Queue,
        screen_w: f32,
        screen_h: f32,
        instances: &[ShadedQuadInstance],
    ) {
        self.write_uniforms(queue, screen_w, screen_h);
        if !instances.is_empty() {
            queue.write_buffer(
                &self.shaded_instance_buf,
                0,
                bytemuck::cast_slice(instances),
            );
        }
        let instance_count = instances.len() as u32;
        self.draw_pass(
            encoder,
            view,
            &self.shaded_pipeline,
            &self.shaded_instance_buf,
            wgpu::LoadOp::Clear(wgpu::Color::BLACK),
            instance_count,
            true,
        );
    }

    pub fn draw_shaded_load(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        queue: &wgpu::Queue,
        screen_w: f32,
        screen_h: f32,
        instances: &[ShadedQuadInstance],
    ) {
        self.write_uniforms(queue, screen_w, screen_h);
        if !instances.is_empty() {
            queue.write_buffer(
                &self.shaded_instance_buf,
                0,
                bytemuck::cast_slice(instances),
            );
        }
        let instance_count = instances.len() as u32;
        self.draw_pass(
            encoder,
            view,
            &self.shaded_pipeline,
            &self.shaded_instance_buf,
            wgpu::LoadOp::Load,
            instance_count,
            false,
        );
    }

    pub fn draw_shaded_masked(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        stencil_view: &wgpu::TextureView,
        queue: &wgpu::Queue,
        screen_w: f32,
        screen_h: f32,
        instances: &[ShadedQuadInstance],
    ) {
        self.write_uniforms(queue, screen_w, screen_h);
        if !instances.is_empty() {
            queue.write_buffer(
                &self.shaded_instance_buf_masked,
                0,
                bytemuck::cast_slice(instances),
            );
        }
        let instance_count = instances.len() as u32;
        self.draw_pass_stenciled(
            encoder,
            view,
            stencil_view,
            &self.shaded_pipeline_stencil_equal,
            &self.shaded_instance_buf_masked,
            instance_count,
        );
    }

    pub fn draw_shaded_outside_cone(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        stencil_view: &wgpu::TextureView,
        queue: &wgpu::Queue,
        screen_w: f32,
        screen_h: f32,
        instances: &[ShadedQuadInstance],
    ) {
        self.write_uniforms(queue, screen_w, screen_h);
        if !instances.is_empty() {
            queue.write_buffer(
                &self.shaded_instance_buf_masked,
                0,
                bytemuck::cast_slice(instances),
            );
        }
        let instance_count = instances.len() as u32;
        self.draw_pass_stenciled(
            encoder,
            view,
            stencil_view,
            &self.shaded_pipeline_stencil_not_equal,
            &self.shaded_instance_buf_masked,
            instance_count,
        );
    }
}
