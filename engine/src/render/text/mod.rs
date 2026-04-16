/// GPU text renderer backed by a fontdue glyph atlas.
///
/// All printable ASCII (32–126) is rasterised once at startup into a single
/// R8Unorm texture. Each frame, `draw()` uploads a flat vertex buffer of
/// coloured character quads and issues a render pass with `LoadOp::Load` so
/// the text lands on top of the existing quad geometry.
mod atlas;
mod types;

pub use types::TextSection;
use types::{FIRST_CHAR, Glyph, LAST_CHAR, MAX_CHARS, NUM_GLYPHS, RASTER_PX, SHADER, Vertex};

const VERTEX_ATTRS: [wgpu::VertexAttribute; 3] =
    wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4];

pub struct TextRenderer {
    pipeline: wgpu::RenderPipeline,
    vertex_buf: wgpu::Buffer,
    index_buf: wgpu::Buffer,
    uniform_buf: wgpu::Buffer,
    uniform_bg: wgpu::BindGroup,
    atlas_bg: wgpu::BindGroup,

    glyphs: [Glyph; NUM_GLYPHS],
    cell_w: u32,
    atlas_w: u32,
    atlas_h: u32,
    ascent: f32,
}

impl TextRenderer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let built = atlas::build();

        // ---- upload atlas texture ----
        let atlas_extent = wgpu::Extent3d {
            width: built.atlas_w,
            height: built.atlas_h,
            depth_or_array_layers: 1,
        };
        let atlas_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("glyph atlas"),
            size: atlas_extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            atlas_tex.as_image_copy(),
            &built.pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(built.atlas_w),
                rows_per_image: Some(built.atlas_h),
            },
            atlas_extent,
        );
        let atlas_view = atlas_tex.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // ---- vertex / index buffers ----
        let vertex_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("text vertices"),
            size: (MAX_CHARS * 4 * std::mem::size_of::<Vertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let indices: Vec<u16> = (0..MAX_CHARS as u16)
            .flat_map(|i| {
                let b = i * 4;
                [b, b + 1, b + 2, b + 1, b + 3, b + 2]
            })
            .collect();
        let index_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("text indices"),
            size: (indices.len() * 2) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&index_buf, 0, bytemuck::cast_slice(&indices));

        // ---- uniform buffer ----
        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("text uniforms"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // ---- bind group layouts ----
        let bgl_uniform = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("text uniform bgl"),
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
        let bgl_atlas = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("text atlas bgl"),
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

        let uniform_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("text uniform bg"),
            layout: &bgl_uniform,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
        });
        let atlas_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("text atlas bg"),
            layout: &bgl_atlas,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        // ---- render pipeline ----
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("text shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("text pipeline layout"),
            bind_group_layouts: &[Some(&bgl_uniform), Some(&bgl_atlas)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("text pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &VERTEX_ATTRS,
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
            index_buf,
            uniform_buf,
            uniform_bg,
            atlas_bg,
            glyphs: built.glyphs,
            cell_w: built.cell_w,
            atlas_w: built.atlas_w,
            atlas_h: built.atlas_h,
            ascent: built.ascent,
        }
    }

    /// Build vertices for all text sections and issue a render pass on top of
    /// whatever is already in `view` (`LoadOp::Load`).
    pub fn draw(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        queue: &wgpu::Queue,
        screen_w: f32,
        screen_h: f32,
        sections: &[TextSection],
    ) {
        queue.write_buffer(
            &self.uniform_buf,
            0,
            bytemuck::cast_slice(&[screen_w, screen_h, 0.0_f32, 0.0_f32]),
        );

        let verts = self.build_vertices(sections);
        if verts.is_empty() {
            return;
        }

        let char_count = verts.len() / 4;
        queue.write_buffer(&self.vertex_buf, 0, bytemuck::cast_slice(&verts));

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("text pass"),
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
        pass.set_bind_group(1, &self.atlas_bg, &[]);
        pass.set_vertex_buffer(0, self.vertex_buf.slice(..));
        pass.set_index_buffer(self.index_buf.slice(..), wgpu::IndexFormat::Uint16);
        pass.draw_indexed(0..(char_count * 6) as u32, 0, 0..1);
    }

    fn build_vertices(&self, sections: &[TextSection]) -> Vec<Vertex> {
        let mut verts = Vec::new();
        let atlas_wf = self.atlas_w as f32;
        let atlas_hf = self.atlas_h as f32;

        for sec in sections {
            let scale = sec.size / RASTER_PX;
            let baseline = sec.y + self.ascent * scale;
            let mut pen_x = sec.x;

            for ch in sec.text.chars() {
                let code = ch as u8;
                if code < FIRST_CHAR || code > LAST_CHAR {
                    pen_x += (self.cell_w as f32) * scale * 0.5;
                    continue;
                }
                let g = &self.glyphs[(code - FIRST_CHAR) as usize];

                let x0 = pen_x + g.bear_x * scale;
                let y0 = baseline - g.bear_y * scale;
                let x1 = x0 + g.bmp_w as f32 * scale;
                let y1 = y0 + g.bmp_h as f32 * scale;

                let u0 = (g.cell_x + 1) as f32 / atlas_wf;
                let v0 = (g.cell_y + 1) as f32 / atlas_hf;
                let u1 = (g.cell_x + 1 + g.bmp_w) as f32 / atlas_wf;
                let v1 = (g.cell_y + 1 + g.bmp_h) as f32 / atlas_hf;

                let c = sec.color;
                verts.push(Vertex {
                    pos: [x0, y0],
                    uv: [u0, v0],
                    color: c,
                });
                verts.push(Vertex {
                    pos: [x1, y0],
                    uv: [u1, v0],
                    color: c,
                });
                verts.push(Vertex {
                    pos: [x0, y1],
                    uv: [u0, v1],
                    color: c,
                });
                verts.push(Vertex {
                    pos: [x1, y1],
                    uv: [u1, v1],
                    color: c,
                });

                pen_x += g.advance * scale;
            }
        }

        verts
    }
}
