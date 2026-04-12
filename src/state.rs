use std::sync::Arc;

use wgpu::{CurrentSurfaceTexture, Device, Queue, Surface, SurfaceConfiguration};
use winit::dpi::PhysicalSize;
use winit::window::Window;

use crate::geometry::{GeoVertex, GeometryRenderer};
use crate::renderer::{QuadInstance, Renderer};
use crate::text::{TextRenderer, TextSection};

pub struct State {
    pub window: Arc<Window>,
    surface:    Surface<'static>,
    device:     Device,
    queue:      Queue,
    config:     SurfaceConfiguration,
    pub size:   PhysicalSize<u32>,
    renderer:   Renderer,
    geo:        GeometryRenderer,
    text:       TextRenderer,
}

impl State {
    pub async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::default();
        let surface  = instance.create_surface(Arc::clone(&window)).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference:       wgpu::PowerPreference::default(),
                compatible_surface:     Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("No suitable GPU adapter found");

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("Failed to create device");

        let caps   = surface.get_capabilities(&adapter);
        let format = caps.formats.iter().find(|f| f.is_srgb()).copied().unwrap_or(caps.formats[0]);

        let config = SurfaceConfiguration {
            usage:                         wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width:                         size.width,
            height:                        size.height,
            present_mode:                  caps.present_modes[0],
            alpha_mode:                    caps.alpha_modes[0],
            view_formats:                  vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let renderer = Renderer::new(&device, format);
        let geo      = GeometryRenderer::new(&device, format);
        let text     = TextRenderer::new(&device, &queue, format);

        Self { window, surface, device, queue, config, size, renderer, geo, text }
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 { return; }
        self.size          = new_size;
        self.config.width  = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);
    }

    /// Draw quads (clears to background), then sight-cone geometry, then text on top.
    pub fn render(&mut self, quads: &[QuadInstance], geo_verts: &[GeoVertex], texts: &[TextSection]) {
        let surface_tex = match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(t) | CurrentSurfaceTexture::Suboptimal(t) => t,
            CurrentSurfaceTexture::Outdated | CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            _ => return,
        };

        let view = surface_tex.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        let sw = self.size.width  as f32;
        let sh = self.size.height as f32;

        // Pass 1: quads (clears to background colour).
        self.renderer.draw(&mut encoder, &view, &self.queue, sw, sh, quads);

        // Pass 2: sight-cone triangle fans (LoadOp::Load — layered on top of quads).
        self.geo.draw(&mut encoder, &view, &self.queue, sw, sh, geo_verts);

        // Pass 3: text (LoadOp::Load — topmost layer).
        self.text.draw(&mut encoder, &view, &self.queue, sw, sh, texts);

        self.queue.submit([encoder.finish()]);
        surface_tex.present();
    }
}
