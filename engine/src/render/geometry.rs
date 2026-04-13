/// Renders arbitrary colored triangles (used for sight cones, fan overlays).
///
/// Each call to `draw` issues a render pass with `LoadOp::Load` so the geometry
/// lands on top of the existing frame without clearing it.
use bytemuck::{Pod, Zeroable};

const SHADER: &str = r#"
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

const MAX_VERTS: u64 = 65_536 * 3; // 65k triangles per frame

/// One coloured vertex for a triangle.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GeoVertex {
    pub pos: [f32; 2],
    pub color: [f32; 4],
}

pub struct GeometryRenderer {
    pipeline: wgpu::RenderPipeline,
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

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("geo bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("geo bg"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("geo pipeline layout"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("geo pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<GeoVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4],
                }],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
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
            uniform_buf,
            bind_group,
        }
    }

    /// Upload `verts` (packed triangle list, 3 per triangle) and draw in a
    /// new render pass that *loads* the existing frame contents.
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
}

// ---------------------------------------------------------------------------
// Helper: build a triangle fan from a center + arc point list
// ---------------------------------------------------------------------------

/// Build a filled circle disk as a triangle fan.
pub fn push_circle_fan(
    out: &mut Vec<GeoVertex>,
    center: (f32, f32),
    radius: f32,
    color: [f32; 4],
    n: usize,
) {
    use std::f32::consts::TAU;
    let arc: Vec<[f32; 2]> = (0..=n)
        .map(|i| {
            let a = i as f32 / n as f32 * TAU;
            [center.0 + a.cos() * radius, center.1 + a.sin() * radius]
        })
        .collect();
    push_cone_fan(out, center, &arc, color);
}

pub fn push_cone_fan(
    out: &mut Vec<GeoVertex>,
    center: (f32, f32),
    arc: &[[f32; 2]],
    color: [f32; 4],
) {
    let c = GeoVertex {
        pos: [center.0, center.1],
        color,
    };
    for i in 1..arc.len() {
        out.push(c);
        out.push(GeoVertex {
            pos: arc[i - 1],
            color,
        });
        out.push(GeoVertex { pos: arc[i], color });
    }
}
