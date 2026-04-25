use crate::render::stencil_read_state;
use super::pipeline::make_pipeline;
use super::shader::SHADER;
use super::types::{ConeVisInstance, GradientQuadInstance, QuadInstance, ShadedQuadInstance};

use bytemuck::Pod;
use wgpu::util::DeviceExt;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const VERTICES: &[[f32; 2]] = &[[-1.0, -1.0], [1.0, -1.0], [-1.0, 1.0], [1.0, 1.0]];
const INDICES: &[u16] = &[0, 1, 2, 1, 3, 2];
const MAX_INSTANCES: u64 = 4096;

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
    /// Per-pixel cone visibility pipeline (stencil Equal for wall occlusion).
    cone_vis_pipeline_equal: wgpu::RenderPipeline,
    vertex_buf: wgpu::Buffer,
    index_buf: wgpu::Buffer,
    instance_buf: wgpu::Buffer,
    instance_buf_masked: wgpu::Buffer,
    gradient_instance_buf: wgpu::Buffer,
    gradient_instance_buf_masked: wgpu::Buffer,
    shaded_instance_buf: wgpu::Buffer,
    shaded_instance_buf_masked: wgpu::Buffer,
    cone_vis_instance_buf: wgpu::Buffer,
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

        let instance_buf = make_instance_buf::<QuadInstance>(device, "instances");
        let instance_buf_masked = make_instance_buf::<QuadInstance>(device, "instances (masked)");
        let gradient_instance_buf =
            make_instance_buf::<GradientQuadInstance>(device, "gradient instances");
        let gradient_instance_buf_masked =
            make_instance_buf::<GradientQuadInstance>(device, "gradient instances (masked)");
        let shaded_instance_buf =
            make_instance_buf::<ShadedQuadInstance>(device, "shaded instances");
        let shaded_instance_buf_masked =
            make_instance_buf::<ShadedQuadInstance>(device, "shaded instances (masked)");

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

        let color_target = wgpu::ColorTargetState {
            format,
            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::ALL,
        };

        let cone_vis_instance_buf =
            make_instance_buf::<ConeVisInstance>(device, "cone vis instances");

        // Vertex buffer layouts — defined inline so the attribute arrays' lifetimes
        // are tied to this function scope, which is what make_pipeline needs.
        let solid_vbl = &[
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<[f32; 2]>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x2],
            },
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<QuadInstance>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &wgpu::vertex_attr_array![1 => Float32x2, 2 => Float32x2, 3 => Float32x4],
            },
        ];
        let gradient_vbl = &[
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<[f32; 2]>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x2],
            },
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<GradientQuadInstance>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &wgpu::vertex_attr_array![
                    1 => Float32x2, 2 => Float32x2, 3 => Float32x4, 4 => Float32x4
                ],
            },
        ];
        let shaded_vbl = &[
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<[f32; 2]>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x2],
            },
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<ShadedQuadInstance>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &wgpu::vertex_attr_array![
                    1 => Float32x2, 2 => Float32x2, 3 => Float32x4,
                    4 => Float32x4, 5 => Float32x4, 6 => Float32x4, 7 => Float32x4
                ],
            },
        ];
        let cone_vis_vbl = &[
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<[f32; 2]>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x2],
            },
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<ConeVisInstance>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &wgpu::vertex_attr_array![
                    1 => Float32x2, 2 => Float32x2, 3 => Float32x4,
                    4 => Float32x2, 5 => Float32x2, 6 => Float32x4, 7 => Float32x4
                ],
            },
        ];

        // Closure so each pipeline call is one line instead of nine arguments repeated.
        let make = |vbl, stencil, vs, fs, label| {
            make_pipeline(device, &pipeline_layout, &shader, vbl, &color_target, stencil, vs, fs, label)
        };

        let eq = Some(stencil_read_state(wgpu::CompareFunction::Equal));
        let ne = Some(stencil_read_state(wgpu::CompareFunction::NotEqual));

        Self {
            pipeline: make(solid_vbl, None, "vs_main", "fs_plain", "quad (plain)"),
            pipeline_stencil_equal: make(solid_vbl, eq.clone(), "vs_main", "fs_plain", "quad (eq)"),
            pipeline_stencil_not_equal: make(solid_vbl, ne.clone(), "vs_main", "fs_plain", "quad (ne)"),
            gradient_pipeline: make(gradient_vbl, None, "vs_gradient", "fs_gradient", "grad (plain)"),
            gradient_pipeline_stencil_equal: make(gradient_vbl, eq.clone(), "vs_gradient", "fs_gradient", "grad (eq)"),
            gradient_pipeline_stencil_not_equal: make(gradient_vbl, ne.clone(), "vs_gradient", "fs_gradient", "grad (ne)"),
            shaded_pipeline: make(shaded_vbl, None, "vs_shaded", "fs_shaded", "shaded (plain)"),
            shaded_pipeline_stencil_equal: make(shaded_vbl, eq.clone(), "vs_shaded", "fs_shaded", "shaded (eq)"),
            shaded_pipeline_stencil_not_equal: make(shaded_vbl, ne, "vs_shaded", "fs_shaded", "shaded (ne)"),
            cone_vis_pipeline_equal: make(cone_vis_vbl, eq, "vs_cone_vis", "fs_cone_vis", "cone vis (eq)"),
            vertex_buf,
            index_buf,
            instance_buf,
            instance_buf_masked,
            gradient_instance_buf,
            gradient_instance_buf_masked,
            shaded_instance_buf,
            shaded_instance_buf_masked,
            cone_vis_instance_buf,
            uniform_buf,
            bind_group,
        }
    }

    // ── Core helpers ──────────────────────────────────────────────────────────

    fn write_uniforms(&self, queue: &wgpu::Queue, sw: f32, sh: f32) {
        queue.write_buffer(
            &self.uniform_buf,
            0,
            bytemuck::bytes_of(&Uniforms { screen_size: [sw, sh], _pad: [0.0; 2] }),
        );
    }

    fn draw_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        pipeline: &wgpu::RenderPipeline,
        instance_buf: &wgpu::Buffer,
        load: wgpu::LoadOp<wgpu::Color>,
        count: u32,
        force_pass: bool,
    ) {
        if count == 0 && !force_pass {
            return;
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("quad pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations { load, store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });
        if count == 0 {
            return;
        }
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buf.slice(..));
        pass.set_vertex_buffer(1, instance_buf.slice(..));
        pass.set_index_buffer(self.index_buf.slice(..), wgpu::IndexFormat::Uint16);
        pass.draw_indexed(0..6, 0, 0..count);
    }

    fn draw_pass_stenciled(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        stencil_view: &wgpu::TextureView,
        pipeline: &wgpu::RenderPipeline,
        instance_buf: &wgpu::Buffer,
        count: u32,
    ) {
        if count == 0 {
            return;
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("quad pass (stenciled)"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
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
        pass.draw_indexed(0..6, 0, 0..count);
    }

    /// Upload `instances` into `buf` then issue an unmasked render pass.
    fn upload_and_draw<T: Pod>(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        queue: &wgpu::Queue,
        sw: f32,
        sh: f32,
        pipeline: &wgpu::RenderPipeline,
        buf: &wgpu::Buffer,
        instances: &[T],
        load: wgpu::LoadOp<wgpu::Color>,
        force_pass: bool,
    ) {
        self.write_uniforms(queue, sw, sh);
        if !instances.is_empty() {
            queue.write_buffer(buf, 0, bytemuck::cast_slice(instances));
        }
        self.draw_pass(encoder, view, pipeline, buf, load, instances.len() as u32, force_pass);
    }

    /// Upload `instances` into `buf` then issue a stencil-tested render pass.
    fn upload_and_draw_stenciled<T: Pod>(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        stencil_view: &wgpu::TextureView,
        queue: &wgpu::Queue,
        sw: f32,
        sh: f32,
        pipeline: &wgpu::RenderPipeline,
        buf: &wgpu::Buffer,
        instances: &[T],
    ) {
        self.write_uniforms(queue, sw, sh);
        if !instances.is_empty() {
            queue.write_buffer(buf, 0, bytemuck::cast_slice(instances));
        }
        self.draw_pass_stenciled(encoder, view, stencil_view, pipeline, buf, instances.len() as u32);
    }

    // ── Public draw API ───────────────────────────────────────────────────────

    pub fn draw(
        &self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView,
        queue: &wgpu::Queue, sw: f32, sh: f32, instances: &[QuadInstance],
    ) {
        self.upload_and_draw(encoder, view, queue, sw, sh, &self.pipeline,
            &self.instance_buf, instances, wgpu::LoadOp::Clear(wgpu::Color::BLACK), true);
    }

    pub fn draw_load(
        &self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView,
        queue: &wgpu::Queue, sw: f32, sh: f32, instances: &[QuadInstance],
    ) {
        self.upload_and_draw(encoder, view, queue, sw, sh, &self.pipeline,
            &self.instance_buf, instances, wgpu::LoadOp::Load, false);
    }

    pub fn draw_inside_cone(
        &self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView,
        stencil_view: &wgpu::TextureView, queue: &wgpu::Queue, sw: f32, sh: f32,
        instances: &[QuadInstance],
    ) {
        self.upload_and_draw_stenciled(encoder, view, stencil_view, queue, sw, sh,
            &self.pipeline_stencil_equal, &self.instance_buf, instances);
    }

    pub fn draw_masked(
        &self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView,
        stencil_view: &wgpu::TextureView, queue: &wgpu::Queue, sw: f32, sh: f32,
        instances: &[QuadInstance],
    ) {
        self.upload_and_draw_stenciled(encoder, view, stencil_view, queue, sw, sh,
            &self.pipeline_stencil_equal, &self.instance_buf_masked, instances);
    }

    pub fn draw_dim_overlay(
        &self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView,
        stencil_view: &wgpu::TextureView, queue: &wgpu::Queue, sw: f32, sh: f32,
        outside_dim: f32,
    ) {
        let alpha = outside_dim.clamp(0.0, 1.0);
        if alpha <= 0.0 {
            return;
        }
        let overlay = [QuadInstance {
            center: [sw * 0.5, sh * 0.5],
            half_size: [sw * 0.5, sh * 0.5],
            color: [0.0, 0.0, 0.0, alpha],
        }];
        self.upload_and_draw_stenciled(encoder, view, stencil_view, queue, sw, sh,
            &self.pipeline_stencil_not_equal, &self.instance_buf, &overlay);
    }

    pub fn draw_gradient(
        &self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView,
        queue: &wgpu::Queue, sw: f32, sh: f32, instances: &[GradientQuadInstance],
    ) {
        self.upload_and_draw(encoder, view, queue, sw, sh, &self.gradient_pipeline,
            &self.gradient_instance_buf, instances, wgpu::LoadOp::Clear(wgpu::Color::BLACK), true);
    }

    pub fn draw_gradient_load(
        &self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView,
        queue: &wgpu::Queue, sw: f32, sh: f32, instances: &[GradientQuadInstance],
    ) {
        self.upload_and_draw(encoder, view, queue, sw, sh, &self.gradient_pipeline,
            &self.gradient_instance_buf, instances, wgpu::LoadOp::Load, false);
    }

    pub fn draw_gradient_masked(
        &self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView,
        stencil_view: &wgpu::TextureView, queue: &wgpu::Queue, sw: f32, sh: f32,
        instances: &[GradientQuadInstance],
    ) {
        self.upload_and_draw_stenciled(encoder, view, stencil_view, queue, sw, sh,
            &self.gradient_pipeline_stencil_equal, &self.gradient_instance_buf_masked, instances);
    }

    pub fn draw_gradient_outside_cone(
        &self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView,
        stencil_view: &wgpu::TextureView, queue: &wgpu::Queue, sw: f32, sh: f32,
        instances: &[GradientQuadInstance],
    ) {
        self.upload_and_draw_stenciled(encoder, view, stencil_view, queue, sw, sh,
            &self.gradient_pipeline_stencil_not_equal, &self.gradient_instance_buf_masked, instances);
    }

    pub fn draw_shaded(
        &self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView,
        queue: &wgpu::Queue, sw: f32, sh: f32, instances: &[ShadedQuadInstance],
    ) {
        self.upload_and_draw(encoder, view, queue, sw, sh, &self.shaded_pipeline,
            &self.shaded_instance_buf, instances, wgpu::LoadOp::Clear(wgpu::Color::BLACK), true);
    }

    pub fn draw_shaded_load(
        &self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView,
        queue: &wgpu::Queue, sw: f32, sh: f32, instances: &[ShadedQuadInstance],
    ) {
        self.upload_and_draw(encoder, view, queue, sw, sh, &self.shaded_pipeline,
            &self.shaded_instance_buf, instances, wgpu::LoadOp::Load, false);
    }

    pub fn draw_shaded_masked(
        &self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView,
        stencil_view: &wgpu::TextureView, queue: &wgpu::Queue, sw: f32, sh: f32,
        instances: &[ShadedQuadInstance],
    ) {
        self.upload_and_draw_stenciled(encoder, view, stencil_view, queue, sw, sh,
            &self.shaded_pipeline_stencil_equal, &self.shaded_instance_buf_masked, instances);
    }

    /// Draw enemy quads with per-pixel cone visibility (stencil Equal for wall occlusion).
    ///
    /// The shader computes V(P) = A(P)·D(P) + halo for each pixel of each enemy
    /// quad, giving soft angular and distance falloff instead of a hard stencil clip.
    pub fn draw_cone_vis_masked(
        &self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView,
        stencil_view: &wgpu::TextureView, queue: &wgpu::Queue, sw: f32, sh: f32,
        instances: &[ConeVisInstance],
    ) {
        self.upload_and_draw_stenciled(encoder, view, stencil_view, queue, sw, sh,
            &self.cone_vis_pipeline_equal, &self.cone_vis_instance_buf, instances);
    }

    pub fn draw_shaded_outside_cone(
        &self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView,
        stencil_view: &wgpu::TextureView, queue: &wgpu::Queue, sw: f32, sh: f32,
        instances: &[ShadedQuadInstance],
    ) {
        self.upload_and_draw_stenciled(encoder, view, stencil_view, queue, sw, sh,
            &self.shaded_pipeline_stencil_not_equal, &self.shaded_instance_buf_masked, instances);
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_instance_buf<T>(device: &wgpu::Device, label: &str) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: MAX_INSTANCES * std::mem::size_of::<T>() as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}
