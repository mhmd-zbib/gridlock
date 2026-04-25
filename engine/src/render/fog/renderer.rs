use super::types::FogUniforms;
use crate::render::stencil_read_state;

const SHADER: &str = r#"
// ─── Layout ────────────────────────────────────────────────────────────────
// FogSource: 32 bytes (vec2 + f32 + f32 + vec2 + f32 + f32_pad)
// FogUniforms: 16-byte header + 8×32 = 272 bytes total

struct FogSource {
    pos:           vec2<f32>,   // offset  0
    cone_range:    f32,         // offset  8
    circle_radius: f32,         // offset 12
    direction:     vec2<f32>,   // offset 16
    half_angle:    f32,         // offset 24
    _pad:          f32,         // offset 28
}

struct FogUniforms {
    screen_size:  vec2<f32>,
    outside_dim:  f32,
    source_count: u32,
    sources:      array<FogSource, 8>,
}

@group(0) @binding(0) var<uniform> u: FogUniforms;

// ─── Vertex: full-screen triangle, no vertex buffer ───────────────────────
@vertex fn vs(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4<f32> {
    var ndc = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 3.0,  1.0),
    );
    return vec4<f32>(ndc[vi], 0.0, 1.0);
}

// ─── Math ─────────────────────────────────────────────────────────────────

// Perlin smootherstep — C² at both ends, no kink.
fn smootherstep(t: f32) -> f32 {
    let c = clamp(t, 0.0, 1.0);
    return c * c * c * (c * (c * 6.0 - 15.0) + 10.0);
}

fn falloff(t: f32) -> f32 {
    return 1.0 - smootherstep(clamp(t, 0.0, 1.0));
}

// Width of the angular fade zone at the cone edges, in radians.
// Increase to make the cone edges softer; decrease for a tighter falloff.
// Clamped to half_angle * 0.4 so it never eats more than 40% of the cone.
const CONE_EDGE_FEATHER: f32 = 0.25; // ~14°

// ─── Per-source light ─────────────────────────────────────────────────────
//
// Wall edges are hard because the stencil buffer (ray-cast cone geometry)
// clips this shader to the exact wall-aware visible region — pixels behind
// walls are never reached.
//
// Cone angular edges are soft: we apply a smootherstep feather over the last
// FEATHER_ANGLE radians before ±half_angle. This works because those pixels
// ARE inside the stencil (they're in open air), so the shader runs there and
// can blend them toward outside_dim. The NotEqual dim pass outside the stencil
// then matches seamlessly.
fn source_light(px: vec2<f32>, s: FogSource) -> f32 {
    let offset = px - s.pos;
    let dist   = length(offset);

    // Near-halo: smooth omnidirectional disk around the player.
    let cr     = max(s.circle_radius, 1.0);
    let circle = falloff(dist / cr);

    // Beam: full brightness from 0–15% of range, cubic fade to zero at range.
    let rng    = max(s.cone_range, 1.0);
    let dt     = dist / rng;
    let radial = falloff(max(0.0, (dt - 0.15) / 0.85));

    // Angular feathering: soft edges at ±half_angle, hard edges at walls.
    // We use cos-space comparisons to avoid acos (expensive).
    var angular: f32 = 1.0;
    if s.half_angle > 0.01 && s.cone_range > 0.0 && dist > 0.5 {
        let feather    = min(CONE_EDGE_FEATHER, s.half_angle * 0.4);
        let cos_inner  = cos(s.half_angle - feather); // fully lit inside this
        let cos_outer  = cos(s.half_angle);            // fully dark at edge
        let dot_val    = dot(normalize(offset), s.direction);
        // t: 0 at inner boundary, 1 at angular edge
        let t = (cos_inner - dot_val) / max(cos_inner - cos_outer, 0.0001);
        angular = 1.0 - smootherstep(clamp(t, 0.0, 1.0));
    }
    let beam = radial * angular;

    return min(1.0, circle + beam);
}

// ─── Fragment — runs only inside the stencil (stencil == 1) ───────────────
//
// Darkness approaches outside_dim at the perimeter, matching the flat
// NotEqual pass outside the stencil — no seam at wall edges.
@fragment fn fs(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
    let px = frag.xy;

    var light: f32 = 0.0;
    for (var i = 0u; i < u.source_count; i++) {
        light = max(light, source_light(px, u.sources[i]));
    }

    let darkness = u.outside_dim * (1.0 - light);
    return vec4<f32>(0.0, 0.0, 0.0, darkness);
}
"#;

pub struct FogRenderer {
    pipeline: wgpu::RenderPipeline,
    uniform_buf: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl FogRenderer {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fog shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });

        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fog uniforms"),
            size: std::mem::size_of::<FogUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fog bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fog bind group"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("fog pipeline layout"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });

        // Stencil Equal: only runs where the ray-cast cone wrote 1 (the
        // player's actual visible region, wall-aware).
        let stencil = Some(stencil_read_state(wgpu::CompareFunction::Equal));

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("fog pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: stencil,
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

        Self { pipeline, uniform_buf, bind_group }
    }

    /// Draw the spotlight gradient over the player's visible region only.
    ///
    /// The stencil Equal test constrains the pass to pixels inside the
    /// ray-cast cone — so light never bleeds through walls.
    /// A companion `draw_dim_overlay` (NotEqual) call in `render_stencil`
    /// covers the exterior with a flat `outside_dim` overlay, and because the
    /// interior gradient reaches exactly `outside_dim` at the stencil boundary,
    /// there is no seam at wall edges.
    pub fn draw(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        stencil_view: &wgpu::TextureView,
        queue: &wgpu::Queue,
        uniforms: &FogUniforms,
    ) {
        queue.write_buffer(&self.uniform_buf, 0, bytemuck::bytes_of(uniforms));

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("fog pass"),
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
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_stencil_reference(1);
        pass.draw(0..3, 0..1);
    }
}
