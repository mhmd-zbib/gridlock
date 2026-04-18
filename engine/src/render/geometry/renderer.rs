use super::pipeline::{make_bgl, make_bind_group, make_geo_pipeline, stencil_read_state};
use super::primitives::{GeoVertex, MAX_VERTS};

pub(super) const SHADER: &str = r#"
struct Uniforms { screen_size: vec2<f32>, _pad: vec2<f32> }
@group(0) @binding(0) var<uniform> u: Uniforms;

struct V { @location(0) pos: vec2<f32>, @location(1) color: vec4<f32> }
struct F { @builtin(position) clip: vec4<f32>, @location(0) color: vec4<f32> }

@vertex fn vs(v: V) -> F {
    let x =  (v.pos.x / u.screen_size.x) * 2.0 - 1.0;
    let y = -(v.pos.y / u.screen_size.y) * 2.0 + 1.0;
    return F(vec4<f32>(x, y, 0.0, 1.0), v.color);
}
@fragment fn fs(v: F) -> @location(0) vec4<f32> { return v.color; }
"#;

pub struct GeometryRenderer {
    pipeline: wgpu::RenderPipeline,
    pipeline_masked: wgpu::RenderPipeline,
    vertex_buf: wgpu::Buffer,
    uniform_buf: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl GeometryRenderer {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("geo shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });

        let vertex_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("geo vertices"),
            size: MAX_VERTS * std::mem::size_of::<GeoVertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("geo uniforms"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bgl = make_bgl(device, "geo bgl", wgpu::ShaderStages::VERTEX);
        let bind_group = make_bind_group(device, &bgl, &uniform_buf, "geo bg");
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("geo pipeline layout"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });

        let color_target = wgpu::ColorTargetState {
            format,
            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::ALL,
        };

        let pipeline = make_geo_pipeline(
            device,
            &layout,
            &shader,
            &color_target,
            None,
            "geo pipeline",
        );
        let pipeline_masked = make_geo_pipeline(
            device,
            &layout,
            &shader,
            &color_target,
            Some(stencil_read_state(wgpu::CompareFunction::Equal)),
            "geo pipeline masked",
        );

        Self {
            pipeline,
            pipeline_masked,
            vertex_buf,
            uniform_buf,
            bind_group,
        }
    }

    pub fn draw(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        queue: &wgpu::Queue,
        screen_w: f32,
        screen_h: f32,
        verts: &[GeoVertex],
    ) {
        if verts.is_empty() {
            return;
        }
        queue.write_buffer(
            &self.uniform_buf,
            0,
            bytemuck::cast_slice(&[screen_w, screen_h, 0.0_f32, 0.0_f32]),
        );
        queue.write_buffer(&self.vertex_buf, 0, bytemuck::cast_slice(verts));

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("geo pass"),
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
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buf.slice(..));
        pass.draw(0..verts.len() as u32, 0..1);
    }

    pub fn draw_masked(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        stencil_view: &wgpu::TextureView,
        queue: &wgpu::Queue,
        screen_w: f32,
        screen_h: f32,
        verts: &[GeoVertex],
    ) {
        if verts.is_empty() {
            return;
        }
        queue.write_buffer(
            &self.uniform_buf,
            0,
            bytemuck::cast_slice(&[screen_w, screen_h, 0.0_f32, 0.0_f32]),
        );
        queue.write_buffer(&self.vertex_buf, 0, bytemuck::cast_slice(verts));

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("geo pass masked"),
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
        pass.set_pipeline(&self.pipeline_masked);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buf.slice(..));
        pass.set_stencil_reference(1);
        pass.draw(0..verts.len() as u32, 0..1);
    }
}
