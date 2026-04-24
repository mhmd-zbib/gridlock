use std::sync::Arc;

use wgpu::{CurrentSurfaceTexture, Device, Queue, Surface, SurfaceConfiguration};
use winit::dpi::PhysicalSize;
use winit::window::Window;

use crate::asset::{AssetHandle, TextureCache};

use super::frame::Frame;
use super::geometry::{GeoVertex, GeometryRenderer, MaskGeometryRenderer};
use super::quad::{GradientQuadInstance, QuadInstance, Renderer, ShadedQuadInstance};
use super::sprite::{SpriteCommand, SpriteRenderer};
use super::text::{TextRenderer, TextSection};

pub struct State {
    pub window: Arc<Window>,
    surface: Surface<'static>,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    pub size: PhysicalSize<u32>,
    renderer: Renderer,
    geo: GeometryRenderer,
    text: TextRenderer,
    sprite: SpriteRenderer,
    texture_cache: TextureCache,
    /// Depth24PlusStencil8 texture used as the FOV visibility mask.
    stencil_tex: wgpu::Texture,
    stencil_view: wgpu::TextureView,
    mask: MaskGeometryRenderer,
}

impl State {
    pub async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(Arc::clone(&window)).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("No suitable GPU adapter found");

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("Failed to create device");

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(caps.formats[0]);

        let config = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width,
            height: size.height,
            present_mode: caps.present_modes[0],
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let renderer = Renderer::new(&device, format);
        let geo = GeometryRenderer::new(&device, format);
        let text = TextRenderer::new(&device, &queue, format);
        let mask = MaskGeometryRenderer::new(&device, format);
        let (stencil_tex, stencil_view) = create_stencil_texture(&device, size.width, size.height);
        let texture_cache = TextureCache::new(&device);
        let sprite = SpriteRenderer::new(&device, format, &texture_cache.bgl);

        Self {
            window,
            surface,
            device,
            queue,
            config,
            size,
            renderer,
            geo,
            text,
            sprite,
            texture_cache,
            stencil_tex,
            stencil_view,
            mask,
        }
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }
        self.size = new_size;
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);
        // Recreate stencil texture to match the new viewport dimensions.
        let (tex, view) = create_stencil_texture(&self.device, new_size.width, new_size.height);
        self.stencil_tex = tex;
        self.stencil_view = view;
    }

    pub fn device(&self) -> &Device {
        &self.device
    }

    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    // ── Asset API ─────────────────────────────────────────────────────────────

    /// Load a PNG from `path`, upload it to the GPU, and return its handle.
    ///
    /// Subsequent calls for the same path are deduplicated: the texture is
    /// only uploaded once and the ref-count is incremented.
    /// Returns `None` if the file is missing or the format is unsupported.
    pub fn load_texture(&mut self, path: &str) -> Option<AssetHandle> {
        self.texture_cache.load(&self.device, &self.queue, path)
    }

    /// Decrement the ref-count for a previously loaded texture.
    /// Call `gc_textures()` to reclaim zero-count entries.
    pub fn release_texture(&mut self, handle: AssetHandle) {
        self.texture_cache.release(handle);
    }

    /// Free all textures whose ref-count has dropped to zero.
    pub fn gc_textures(&mut self) {
        self.texture_cache.collect_garbage();
    }

    // ── Render ────────────────────────────────────────────────────────────────

    /// Render a frame.
    ///
    /// `mask_verts`      — cone geometry written into the stencil buffer.
    ///   Empty slice = no masking (menus use the plain unmasked path).
    ///
    /// `scene_quads`     — base quads (UI / non-wall world quads / scene markers).
    ///   Outside the cone they receive the `outside_dim` dark overlay.
    ///
    /// `masked_quads`    — non-world entities hidden completely outside the cone.
    ///
    /// `scene_floor_sprites` — textured floor quads (below props and walls).
    /// `scene_prop_sprites`  — textured prop quads (below walls).
    /// `wall_quads`          — wall quads composited above world quads/sprites.
    ///
    /// `outside_dim`     — black overlay alpha applied only outside the cone.
    ///
    /// `geo_verts`       — overlays always visible on top (UI/debug visuals).
    /// `masked_geo_verts`— overlays hidden outside the cone (impacts/traces).
    pub fn render_frame(&mut self, frame: &Frame) {
        self.sprite.update_lighting(&self.queue, &frame.lighting);
        self.render(
            &frame.fov_mask,
            &frame.scene_quads,
            &frame.scene_gradient_quads,
            &frame.scene_shaded_quads,
            &frame.masked_quads,
            &frame.floor_sprites,
            &frame.prop_sprites,
            &frame.wall_quads,
            &frame.geo,
            &frame.masked_geo,
            &frame.texts,
            frame.outside_dim,
        );
    }

    pub fn render(
        &mut self,
        mask_verts: &[GeoVertex],
        scene_quads: &[QuadInstance],
        scene_gradient_quads: &[GradientQuadInstance],
        scene_shaded_quads: &[ShadedQuadInstance],
        masked_quads: &[QuadInstance],
        scene_floor_sprites: &[SpriteCommand],
        scene_prop_sprites: &[SpriteCommand],
        wall_quads: &[QuadInstance],
        geo_verts: &[GeoVertex],
        masked_geo_verts: &[GeoVertex],
        texts: &[TextSection],
        outside_dim: f32,
    ) {
        self.render_with_overlay(
            mask_verts,
            scene_quads,
            scene_gradient_quads,
            scene_shaded_quads,
            masked_quads,
            scene_floor_sprites,
            scene_prop_sprites,
            wall_quads,
            geo_verts,
            masked_geo_verts,
            texts,
            outside_dim,
            |_, _, _, _, _| {},
        );
    }

    pub fn render_with_overlay<F>(
        &mut self,
        mask_verts: &[GeoVertex],
        scene_quads: &[QuadInstance],
        scene_gradient_quads: &[GradientQuadInstance],
        scene_shaded_quads: &[ShadedQuadInstance],
        masked_quads: &[QuadInstance],
        scene_floor_sprites: &[SpriteCommand],
        scene_prop_sprites: &[SpriteCommand],
        wall_quads: &[QuadInstance],
        geo_verts: &[GeoVertex],
        masked_geo_verts: &[GeoVertex],
        texts: &[TextSection],
        outside_dim: f32,
        overlay: F,
    ) where
        F: FnOnce(
            &wgpu::Device,
            &wgpu::Queue,
            &mut wgpu::CommandEncoder,
            &wgpu::TextureView,
            [u32; 2],
        ),
    {
        let surface_tex = match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(t) | CurrentSurfaceTexture::Suboptimal(t) => t,
            CurrentSurfaceTexture::Outdated | CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            _ => return,
        };

        let view = surface_tex
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        let sw = self.size.width as f32;
        let sh = self.size.height as f32;

        if mask_verts.is_empty() {
            self.render_unmasked(
                &mut encoder, &view, sw, sh,
                scene_quads, scene_gradient_quads, scene_shaded_quads,
                scene_floor_sprites, scene_prop_sprites, wall_quads, masked_geo_verts,
            );
        } else {
            self.render_stencil(
                &mut encoder, &view, sw, sh,
                mask_verts, scene_quads, scene_gradient_quads, scene_shaded_quads,
                masked_quads, scene_floor_sprites, scene_prop_sprites, wall_quads,
                masked_geo_verts, outside_dim,
            );
        }

        self.geo.draw(&mut encoder, &view, &self.queue, sw, sh, geo_verts);
        self.text.draw(&mut encoder, &view, &self.queue, sw, sh, texts);

        overlay(&self.device, &self.queue, &mut encoder, &view, [self.size.width, self.size.height]);

        self.queue.submit([encoder.finish()]);
        surface_tex.present();
    }

    /// Render pass for menus / lobby: no vision-cone masking.
    #[allow(clippy::too_many_arguments)]
    fn render_unmasked(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        sw: f32,
        sh: f32,
        scene_quads: &[QuadInstance],
        scene_gradient_quads: &[GradientQuadInstance],
        scene_shaded_quads: &[ShadedQuadInstance],
        scene_floor_sprites: &[SpriteCommand],
        scene_prop_sprites: &[SpriteCommand],
        wall_quads: &[QuadInstance],
        geo_verts: &[GeoVertex],
    ) {
        self.renderer.draw(encoder, view, &self.queue, sw, sh, scene_quads);
        self.renderer.draw_gradient_load(encoder, view, &self.queue, sw, sh, scene_gradient_quads);
        self.renderer.draw_shaded_load(encoder, view, &self.queue, sw, sh, scene_shaded_quads);
        self.sprite.draw(encoder, view, &self.queue, sw, sh, scene_floor_sprites, &self.texture_cache);
        self.sprite.draw(encoder, view, &self.queue, sw, sh, scene_prop_sprites, &self.texture_cache);
        self.renderer.draw_load(encoder, view, &self.queue, sw, sh, wall_quads);
        self.geo.draw(encoder, view, &self.queue, sw, sh, geo_verts);
    }

    /// Render pass for gameplay: stencil-masked vision cone.
    #[allow(clippy::too_many_arguments)]
    fn render_stencil(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        sw: f32,
        sh: f32,
        mask_verts: &[GeoVertex],
        scene_quads: &[QuadInstance],
        scene_gradient_quads: &[GradientQuadInstance],
        scene_shaded_quads: &[ShadedQuadInstance],
        masked_quads: &[QuadInstance],
        scene_floor_sprites: &[SpriteCommand],
        scene_prop_sprites: &[SpriteCommand],
        wall_quads: &[QuadInstance],
        masked_geo_verts: &[GeoVertex],
        outside_dim: f32,
    ) {
        // Pass 1: write cone into stencil buffer; clear colour to black.
        self.mask.draw(encoder, view, &self.stencil_view, &self.queue, sw, sh, mask_verts);
        // Pass 2: scene geometry (visible everywhere, dimmed outside cone in pass 3).
        self.renderer.draw_load(encoder, view, &self.queue, sw, sh, scene_quads);
        self.renderer.draw_gradient_load(encoder, view, &self.queue, sw, sh, scene_gradient_quads);
        self.renderer.draw_shaded_load(encoder, view, &self.queue, sw, sh, scene_shaded_quads);
        // Pass 2.5: world sprites, floor below props.
        self.sprite.draw(encoder, view, &self.queue, sw, sh, scene_floor_sprites, &self.texture_cache);
        self.sprite.draw(encoder, view, &self.queue, sw, sh, scene_prop_sprites, &self.texture_cache);
        // Pass 2.75: walls above sprites.
        self.renderer.draw_load(encoder, view, &self.queue, sw, sh, wall_quads);
        // Pass 3: darken everything outside the cone.
        self.renderer.draw_dim_overlay(encoder, view, &self.stencil_view, &self.queue, sw, sh, outside_dim);
        // Pass 4: entities — hidden completely outside the cone.
        self.renderer.draw_masked(encoder, view, &self.stencil_view, &self.queue, sw, sh, masked_quads);
        // Pass 5: geometry overlays masked by cone (impacts, traces).
        self.geo.draw_masked(encoder, view, &self.stencil_view, &self.queue, sw, sh, masked_geo_verts);
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn create_stencil_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("stencil"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth24PlusStencil8,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
    (tex, view)
}
