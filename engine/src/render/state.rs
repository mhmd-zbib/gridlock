use std::sync::Arc;

use wgpu::{CurrentSurfaceTexture, Device, Queue, Surface, SurfaceConfiguration};
use winit::dpi::PhysicalSize;
use winit::window::Window;

use super::geometry::{GeoVertex, GeometryRenderer, MaskGeometryRenderer};
use super::quad::{QuadInstance, Renderer};
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

    /// Render a frame.
    ///
    /// `mask_verts` — cone geometry written into the stencil buffer.
    ///   Empty slice = no masking (menus use the plain unmasked path).
    ///
    /// `scene_quads` — walls, props, floor.  Always fully visible;
    ///   outside the cone they receive the `outside_dim` dark overlay.
    ///
    /// `masked_quads` — non-world entities hidden completely outside the cone.
    ///
    /// `outside_dim` — alpha of the black overlay applied outside the cone
    ///   (0.0 = no dim, 1.0 = full black).  Ignored when `mask_verts` is empty.
    ///
    /// `geo_verts` — overlays always visible on top (UI/debug visuals).
    /// `masked_geo_verts` — overlays hidden outside the cone (impacts/traces).
    pub fn render(
        &mut self,
        mask_verts: &[GeoVertex],
        scene_quads: &[QuadInstance],
        masked_quads: &[QuadInstance],
        geo_verts: &[GeoVertex],
        masked_geo_verts: &[GeoVertex],
        texts: &[TextSection],
        outside_dim: f32,
    ) {
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
            // ── Unmasked path (menus, lobby, etc.) ────────────────────────────
            // All quads drawn normally; masked_quads ignored.
            self.renderer
                .draw(&mut encoder, &view, &self.queue, sw, sh, scene_quads);
            self.geo
                .draw(&mut encoder, &view, &self.queue, sw, sh, masked_geo_verts);
        } else {
            // ── Stencil path (gameplay) ───────────────────────────────────────
            // Pass 1: write cone to stencil=1; clear colour to black.
            self.mask.draw(
                &mut encoder,
                &view,
                &self.stencil_view,
                &self.queue,
                sw,
                sh,
                mask_verts,
            );
            // Pass 2: scene quads (walls, props, floor) — always visible,
            // no stencil test.  LoadOp::Load preserves the black background.
            self.renderer
                .draw_load(&mut encoder, &view, &self.queue, sw, sh, scene_quads);
            // Pass 3: masked quads (non-world entities) — hidden outside the cone.
            self.renderer.draw_masked(
                &mut encoder,
                &view,
                &self.stencil_view,
                &self.queue,
                sw,
                sh,
                masked_quads,
            );
            // Pass 4: dim overlay — darkens scene quads outside the cone.
            self.renderer.draw_dim_overlay(
                &mut encoder,
                &view,
                &self.stencil_view,
                &self.queue,
                sw,
                sh,
                outside_dim,
            );
            // Pass 5: masked overlays (impacts, traces, particles).
            self.geo.draw_masked(
                &mut encoder,
                &view,
                &self.stencil_view,
                &self.queue,
                sw,
                sh,
                masked_geo_verts,
            );
        }

        // Pass 6: unmasked overlays (cone tints, room debug overlays).
        self.geo
            .draw(&mut encoder, &view, &self.queue, sw, sh, geo_verts);

        // Pass 7: text (topmost layer).
        self.text
            .draw(&mut encoder, &view, &self.queue, sw, sh, texts);

        self.queue.submit([encoder.finish()]);
        surface_tex.present();
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
