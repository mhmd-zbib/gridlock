/// GPU text renderer backed by a fontdue glyph atlas.
///
/// All printable ASCII (32–126) is rasterised once at startup into a single
/// R8Unorm texture. Each frame, `draw()` uploads a flat vertex buffer of
/// coloured character quads and issues a render pass with `LoadOp::Load` so
/// the text lands on top of the existing quad geometry.
use bytemuck::{Pod, Zeroable};
use fontdue::{Font, FontSettings};

// ---------------------------------------------------------------------------
// Public text-section type
// ---------------------------------------------------------------------------

pub struct TextSection {
    /// Top-left corner in screen pixels (y down).
    pub x:     f32,
    pub y:     f32,
    pub text:  String,
    /// Rendered line-height in screen pixels.  The atlas is baked at
    /// `RASTER_PX`; other sizes are scaled at the vertex level.
    pub size:  f32,
    pub color: [f32; 4],
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const RASTER_PX:  f32 = 20.0; // px used when rasterising into atlas
const FIRST_CHAR: u8  = 32;   // space
const LAST_CHAR:  u8  = 126;  // ~
const NUM_GLYPHS: usize = (LAST_CHAR - FIRST_CHAR + 1) as usize; // 95

const ATLAS_COLS: u32 = 16;   // glyphs per atlas row
const ATLAS_ROWS: u32 = 6;    // enough rows for 95 glyphs

const MAX_CHARS: usize = 8192;

// ---------------------------------------------------------------------------
// Internal types
// ---------------------------------------------------------------------------

/// Per-glyph atlas metadata (all in atlas-pixel coordinates).
#[derive(Clone, Copy, Default)]
struct Glyph {
    /// Top-left pixel of this glyph's cell in the atlas.
    cell_x: u32,
    cell_y: u32,
    /// Actual rasterised bitmap size (may be smaller than cell).
    bmp_w:  u32,
    bmp_h:  u32,
    /// Offset from pen position to bitmap top-left (in raster-px units).
    bear_x: f32,
    bear_y: f32, // positive = above baseline
    /// How far to advance the pen after this glyph.
    advance: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    pos:   [f32; 2],
    uv:    [f32; 2],
    color: [f32; 4],
}

// ---------------------------------------------------------------------------
// Shader
// ---------------------------------------------------------------------------

const SHADER: &str = r#"
struct Uniforms {
    screen_size: vec2<f32>,
    _pad: vec2<f32>,
}
@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(1) @binding(0) var t_atlas:  texture_2d<f32>;
@group(1) @binding(1) var s_atlas:  sampler;

struct VIn  { @location(0) pos: vec2<f32>, @location(1) uv: vec2<f32>, @location(2) color: vec4<f32> }
struct VOut { @builtin(position) clip: vec4<f32>, @location(0) uv: vec2<f32>, @location(1) color: vec4<f32> }

@vertex fn vs(v: VIn) -> VOut {
    let ndc_x =  (v.pos.x / uniforms.screen_size.x) * 2.0 - 1.0;
    let ndc_y = -(v.pos.y / uniforms.screen_size.y) * 2.0 + 1.0;
    return VOut(vec4<f32>(ndc_x, ndc_y, 0.0, 1.0), v.uv, v.color);
}

@fragment fn fs(v: VOut) -> @location(0) vec4<f32> {
    let alpha = textureSample(t_atlas, s_atlas, v.uv).r;
    return vec4<f32>(v.color.rgb, v.color.a * alpha);
}
"#;

// ---------------------------------------------------------------------------
// TextRenderer
// ---------------------------------------------------------------------------

pub struct TextRenderer {
    pipeline:    wgpu::RenderPipeline,
    vertex_buf:  wgpu::Buffer,
    index_buf:   wgpu::Buffer,
    uniform_buf: wgpu::Buffer,
    uniform_bg:  wgpu::BindGroup,
    atlas_bg:    wgpu::BindGroup,

    glyphs:    [Glyph; NUM_GLYPHS],
    cell_w:  u32,
    atlas_w: u32,
    atlas_h:   u32,
    ascent:    f32, // max pixels above baseline across all glyphs
}

impl TextRenderer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let font_bytes = load_font_bytes();
        let font = Font::from_bytes(font_bytes.as_slice(), FontSettings::default())
            .expect("failed to parse font");

        // ---- rasterise all glyphs to find max cell size ----
        let raw: Vec<(fontdue::Metrics, Vec<u8>)> = (FIRST_CHAR..=LAST_CHAR)
            .map(|c| font.rasterize(c as char, RASTER_PX))
            .collect();

        let cell_w = raw.iter().map(|(m, _)| m.width  as u32).max().unwrap_or(12) + 2;
        let cell_h = raw.iter().map(|(m, _)| m.height as u32).max().unwrap_or(20) + 2;
        let ascent  = raw.iter().map(|(m, _)| m.bounds.ymin + m.height as f32).fold(0.0_f32, f32::max);

        let atlas_w = cell_w * ATLAS_COLS;
        let atlas_h = cell_h * ATLAS_ROWS;

        // ---- pack glyphs into R8 atlas ----
        let mut atlas_data = vec![0u8; (atlas_w * atlas_h) as usize];
        let mut glyphs = [Glyph::default(); NUM_GLYPHS];

        for (i, (metrics, bitmap)) in raw.iter().enumerate() {
            let col = (i as u32) % ATLAS_COLS;
            let row = (i as u32) / ATLAS_COLS;
            let cx  = col * cell_w;
            let cy  = row * cell_h;

            // Copy bitmap into atlas (pad 1px inside cell).
            let bx = cx + 1;
            let by = cy + 1;
            for y in 0..metrics.height {
                for x in 0..metrics.width {
                    let src = bitmap[y * metrics.width + x];
                    let dst = (by + y as u32) * atlas_w + (bx + x as u32);
                    atlas_data[dst as usize] = src;
                }
            }

            glyphs[i] = Glyph {
                cell_x:  cx,
                cell_y:  cy,
                bmp_w:   metrics.width  as u32,
                bmp_h:   metrics.height as u32,
                bear_x:  metrics.bounds.xmin,
                bear_y:  metrics.bounds.ymin + metrics.height as f32,
                advance: metrics.advance_width,
            };
        }

        // ---- upload atlas texture ----
        let atlas_extent = wgpu::Extent3d { width: atlas_w, height: atlas_h, depth_or_array_layers: 1 };
        let atlas_tex = device.create_texture(&wgpu::TextureDescriptor {
            label:           Some("glyph atlas"),
            size:            atlas_extent,
            mip_level_count: 1,
            sample_count:    1,
            dimension:       wgpu::TextureDimension::D2,
            format:          wgpu::TextureFormat::R8Unorm,
            usage:           wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats:    &[],
        });
        queue.write_texture(
            atlas_tex.as_image_copy(),
            &atlas_data,
            wgpu::TexelCopyBufferLayout {
                offset:         0,
                bytes_per_row:  Some(atlas_w),
                rows_per_image: Some(atlas_h),
            },
            atlas_extent,
        );
        let atlas_view = atlas_tex.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter:     wgpu::FilterMode::Linear,
            min_filter:     wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // ---- vertex / index buffers ----
        let vertex_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label:              Some("text vertices"),
            size:               (MAX_CHARS * 4 * std::mem::size_of::<Vertex>()) as u64,
            usage:              wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Pre-build index buffer: [0,1,2, 1,3,2] per quad.
        let indices: Vec<u16> = (0..MAX_CHARS as u16)
            .flat_map(|i| {
                let b = i * 4;
                [b, b+1, b+2, b+1, b+3, b+2]
            })
            .collect();
        let index_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label:              Some("text indices"),
            size:               (indices.len() * 2) as u64,
            usage:              wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&index_buf, 0, bytemuck::cast_slice(&indices));

        // ---- uniform buffer (screen size) ----
        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label:              Some("text uniforms"),
            size:               16,
            usage:              wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // ---- bind group layouts ----
        let bgl_uniform = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label:   Some("text uniform bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding:    0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty:         wgpu::BindingType::Buffer {
                    ty:                 wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size:   None,
                },
                count: None,
            }],
        });
        let bgl_atlas = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label:   Some("text atlas bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding:    0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty:         wgpu::BindingType::Texture {
                        sample_type:    wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled:   false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding:    1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty:         wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let uniform_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label:   Some("text uniform bg"),
            layout:  &bgl_uniform,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: uniform_buf.as_entire_binding() }],
        });
        let atlas_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label:   Some("text atlas bg"),
            layout:  &bgl_atlas,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&atlas_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) },
            ],
        });

        // ---- render pipeline ----
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label:  Some("text shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label:              Some("text pipeline layout"),
            bind_group_layouts: &[Some(&bgl_uniform), Some(&bgl_atlas)],
            immediate_size:     0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label:  Some("text pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module:              &shader,
                entry_point:         Some("vs"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode:    wgpu::VertexStepMode::Vertex,
                    attributes:   &wgpu::vertex_attr_array![
                        0 => Float32x2, // pos
                        1 => Float32x2, // uv
                        2 => Float32x4, // color
                    ],
                }],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample:   wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module:              &shader,
                entry_point:         Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend:      Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache:          None,
        });

        Self {
            pipeline, vertex_buf, index_buf, uniform_buf, uniform_bg, atlas_bg,
            glyphs, cell_w, atlas_w, atlas_h, ascent,
        }
    }

    /// Build vertices for all text sections and issue a render pass on top of
    /// whatever is already in `view` (`LoadOp::Load`).
    pub fn draw(
        &self,
        encoder:  &mut wgpu::CommandEncoder,
        view:     &wgpu::TextureView,
        queue:    &wgpu::Queue,
        screen_w: f32,
        screen_h: f32,
        sections: &[TextSection],
    ) {
        // Upload screen size.
        queue.write_buffer(&self.uniform_buf, 0, bytemuck::cast_slice(&[screen_w, screen_h, 0.0_f32, 0.0_f32]));

        // Build vertex buffer.
        let mut verts: Vec<Vertex> = Vec::new();
        let atlas_wf = self.atlas_w as f32;
        let atlas_hf = self.atlas_h as f32;

        for sec in sections {
            let scale = sec.size / RASTER_PX;
            // Pen starts at sec.x; baseline is sec.y + ascent*scale.
            let baseline = sec.y + self.ascent * scale;
            let mut pen_x = sec.x;

            for ch in sec.text.chars() {
                let code = ch as u8;
                if code < FIRST_CHAR || code > LAST_CHAR {
                    pen_x += (self.cell_w as f32) * scale * 0.5;
                    continue;
                }
                let g = &self.glyphs[(code - FIRST_CHAR) as usize];

                // Screen-space quad corners.
                let x0 = pen_x + g.bear_x * scale;
                let y0 = baseline - g.bear_y * scale;
                let x1 = x0 + g.bmp_w as f32 * scale;
                let y1 = y0 + g.bmp_h as f32 * scale;

                // Atlas UV corners (using the 1px-padded bitmap region).
                let u0 = (g.cell_x + 1) as f32 / atlas_wf;
                let v0 = (g.cell_y + 1) as f32 / atlas_hf;
                let u1 = (g.cell_x + 1 + g.bmp_w) as f32 / atlas_wf;
                let v1 = (g.cell_y + 1 + g.bmp_h) as f32 / atlas_hf;

                let c = sec.color;
                verts.push(Vertex { pos: [x0, y0], uv: [u0, v0], color: c });
                verts.push(Vertex { pos: [x1, y0], uv: [u1, v0], color: c });
                verts.push(Vertex { pos: [x0, y1], uv: [u0, v1], color: c });
                verts.push(Vertex { pos: [x1, y1], uv: [u1, v1], color: c });

                pen_x += g.advance * scale;
            }
        }

        if verts.is_empty() { return; }

        let char_count = verts.len() / 4;
        queue.write_buffer(&self.vertex_buf, 0, bytemuck::cast_slice(&verts));

        // Render pass — LoadOp::Load so quads underneath are preserved.
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("text pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                depth_slice:    None,
                ops: wgpu::Operations {
                    load:  wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.uniform_bg, &[]);
        pass.set_bind_group(1, &self.atlas_bg,   &[]);
        pass.set_vertex_buffer(0, self.vertex_buf.slice(..));
        pass.set_index_buffer(self.index_buf.slice(..), wgpu::IndexFormat::Uint16);
        pass.draw_indexed(0..(char_count * 6) as u32, 0, 0..1);
    }
}

// ---------------------------------------------------------------------------
// Font loading — checks project assets then common system paths
// ---------------------------------------------------------------------------

fn load_font_bytes() -> Vec<u8> {
    let candidates: &[&str] = &[
        "assets/font.ttf",
        // macOS
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "/Library/Fonts/Arial.ttf",
        "/System/Library/Fonts/Monaco.ttf",
        // Linux
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/TTF/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        // Windows
        "C:/Windows/Fonts/arial.ttf",
        "C:/Windows/Fonts/calibri.ttf",
    ];
    for &path in candidates {
        if let Ok(data) = std::fs::read(path) {
            println!("[text] font loaded from {path}");
            return data;
        }
    }
    panic!("[text] no font found — place a .ttf file at assets/font.ttf");
}
