use std::collections::HashMap;
use std::io::Cursor;

use super::handle::AssetHandle;

struct CacheEntry {
    bind_group: wgpu::BindGroup,
    ref_count: u32,
}

/// GPU texture cache.
///
/// Owns the bind-group layout used for all sprite texture bindings, so the
/// `SpriteRenderer` can reference it during pipeline construction.
///
/// # Lifecycle
///
/// `load()` uploads the texture on first request and increments the ref-count
/// on subsequent calls for the same path.  `release()` decrements the count;
/// call `collect_garbage()` to actually free zero-count entries.
pub struct TextureCache {
    pub(crate) bgl: wgpu::BindGroupLayout,
    entries: HashMap<AssetHandle, CacheEntry>,
}

impl TextureCache {
    pub fn new(device: &wgpu::Device) -> Self {
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sprite texture bgl"),
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
        Self {
            bgl,
            entries: HashMap::new(),
        }
    }

    /// Decode a PNG from disk, upload it to the GPU, and return its handle.
    ///
    /// If the path was already loaded the ref-count is incremented and the
    /// existing handle is returned immediately (no re-upload).
    /// Returns `None` if the file is missing or cannot be decoded.
    pub fn load(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: &str,
    ) -> Option<AssetHandle> {
        let handle = AssetHandle::from_path(path);

        if let Some(entry) = self.entries.get_mut(&handle) {
            entry.ref_count += 1;
            return Some(handle);
        }

        let bytes = std::fs::read(path).ok()?;
        let (rgba, width, height) = decode_png(&bytes)?;

        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(path),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            texture.as_image_copy(),
            &rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(path),
            layout: &self.bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        self.entries.insert(handle, CacheEntry { bind_group, ref_count: 1 });
        Some(handle)
    }

    /// Decrement the ref-count for the given handle.
    /// Call `collect_garbage()` to reclaim zero-count entries.
    pub fn release(&mut self, handle: AssetHandle) {
        if let Some(entry) = self.entries.get_mut(&handle) {
            entry.ref_count = entry.ref_count.saturating_sub(1);
        }
    }

    /// Drop all entries with ref-count == 0, freeing their GPU resources.
    pub fn collect_garbage(&mut self) {
        self.entries.retain(|_, entry| entry.ref_count > 0);
    }

    pub(crate) fn bind_group(&self, handle: AssetHandle) -> Option<&wgpu::BindGroup> {
        self.entries.get(&handle).map(|e| &e.bind_group)
    }
}

// ---------------------------------------------------------------------------
// PNG decoding
// ---------------------------------------------------------------------------

fn decode_png(bytes: &[u8]) -> Option<(Vec<u8>, u32, u32)> {
    let decoder = png::Decoder::new(Cursor::new(bytes));
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;

    let width = info.width;
    let height = info.height;
    let pixel_count = (width * height) as usize;

    let rgba = match info.color_type {
        png::ColorType::Rgba => buf[..pixel_count * 4].to_vec(),
        png::ColorType::Rgb => {
            let mut out = Vec::with_capacity(pixel_count * 4);
            for rgb in buf[..pixel_count * 3].chunks_exact(3) {
                out.push(rgb[0]);
                out.push(rgb[1]);
                out.push(rgb[2]);
                out.push(255);
            }
            out
        }
        png::ColorType::GrayscaleAlpha => {
            let mut out = Vec::with_capacity(pixel_count * 4);
            for ga in buf[..pixel_count * 2].chunks_exact(2) {
                out.extend_from_slice(&[ga[0], ga[0], ga[0], ga[1]]);
            }
            out
        }
        png::ColorType::Grayscale => {
            let mut out = Vec::with_capacity(pixel_count * 4);
            for &g in buf[..pixel_count].iter() {
                out.extend_from_slice(&[g, g, g, 255]);
            }
            out
        }
        _ => return None,
    };

    Some((rgba, width, height))
}
