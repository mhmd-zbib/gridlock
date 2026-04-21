use std::any::TypeId;
use std::marker::PhantomData;

use ab_glyph::PxScale;
use cosmic_text::{AttrsOwned, Metrics as CosmicMetrics};
use guillotiere::{AtlasAllocator, size2};
use image::{ImageFormat, RgbaImage};
use rustybuzz::Direction;

pub struct HudDependencies {
    pub clear_color: wgpu::Color,
    pub text_metrics: CosmicMetrics,
    pub glyph_metrics: glyphon::Metrics,
    pub atlas: AtlasAllocator,
    pub icon_sheet: RgbaImage,
    pub icon_format: ImageFormat,
    pub glyph_scale: PxScale,
    pub shaping_direction: Direction,
    pub glyphon_resolution_type: TypeId,
    pub _cosmic_attrs: PhantomData<AttrsOwned>,
}

impl HudDependencies {
    pub fn tactical_defaults() -> Self {
        Self {
            clear_color: wgpu::Color {
                r: 0.035,
                g: 0.047,
                b: 0.070,
                a: 1.0,
            },
            text_metrics: CosmicMetrics::new(18.0, 24.0),
            glyph_metrics: glyphon::Metrics::new(18.0, 24.0),
            atlas: AtlasAllocator::new(size2(2048, 2048)),
            icon_sheet: RgbaImage::new(1, 1),
            icon_format: ImageFormat::Png,
            glyph_scale: PxScale::from(18.0),
            shaping_direction: Direction::LeftToRight,
            glyphon_resolution_type: TypeId::of::<glyphon::Resolution>(),
            _cosmic_attrs: PhantomData,
        }
    }
}
