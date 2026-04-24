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
        let (rgba, width, height) = decode_image(path, &bytes)?;
        let bind_group = upload_texture(device, queue, &self.bgl, path, &rgba, width, height);

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
// GPU upload
// ---------------------------------------------------------------------------

fn upload_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bgl: &wgpu::BindGroupLayout,
    label: &str,
    rgba: &[u8],
    width: u32,
    height: u32,
) -> wgpu::BindGroup {
    let size = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
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
        rgba,
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

    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout: bgl,
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
    })
}

// ---------------------------------------------------------------------------
// Image decoding (PNG / JPEG)
// ---------------------------------------------------------------------------

fn decode_image(path: &str, bytes: &[u8]) -> Option<(Vec<u8>, u32, u32)> {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase());

    match ext.as_deref() {
        Some("jpg") | Some("jpeg") => decode_jpeg(bytes),
        _ => decode_png(bytes),
    }
}

fn decode_jpeg(bytes: &[u8]) -> Option<(Vec<u8>, u32, u32)> {
    let mut decoder = jpeg_decoder::Decoder::new(Cursor::new(bytes));
    let pixels = decoder.decode().ok()?;
    let info = decoder.info()?;
    let width = info.width as u32;
    let height = info.height as u32;
    let pixel_count = (width * height) as usize;

    let rgba = match info.pixel_format {
        jpeg_decoder::PixelFormat::RGB24 => rgb_to_rgba(&pixels, pixel_count),
        jpeg_decoder::PixelFormat::L8 => gray_to_rgba(&pixels),
        _ => return None,
    };

    Some((rgba, width, height))
}

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
        png::ColorType::Rgb => rgb_to_rgba(&buf[..pixel_count * 3], pixel_count),
        png::ColorType::GrayscaleAlpha => gray_alpha_to_rgba(&buf[..pixel_count * 2]),
        png::ColorType::Grayscale => gray_to_rgba(&buf[..pixel_count]),
        _ => return None,
    };

    Some((rgba, width, height))
}

// ---------------------------------------------------------------------------
// Pixel format converters
// ---------------------------------------------------------------------------

fn rgb_to_rgba(src: &[u8], pixel_count: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(pixel_count * 4);
    for rgb in src.chunks_exact(3) {
        out.push(rgb[0]);
        out.push(rgb[1]);
        out.push(rgb[2]);
        out.push(255);
    }
    out
}

fn gray_to_rgba(src: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(src.len() * 4);
    for &g in src {
        out.extend_from_slice(&[g, g, g, 255]);
    }
    out
}

fn gray_alpha_to_rgba(src: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(src.len() * 2);
    for ga in src.chunks_exact(2) {
        out.extend_from_slice(&[ga[0], ga[0], ga[0], ga[1]]);
    }
    out
}
