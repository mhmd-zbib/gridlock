use bytemuck::{Pod, Zeroable};

// ── Constants ────────────────────────────────────────────────────────────────

pub(super) const RASTER_PX: f32 = 20.0;
pub(super) const FIRST_CHAR: u8 = 32; // space
pub(super) const LAST_CHAR: u8 = 126; // ~
pub(super) const NUM_GLYPHS: usize = (LAST_CHAR - FIRST_CHAR + 1) as usize; // 95

pub(super) const ATLAS_COLS: u32 = 16;
pub(super) const ATLAS_ROWS: u32 = 6;

pub(super) const MAX_CHARS: usize = 8192;

// ── Public types ─────────────────────────────────────────────────────────────

/// A run of text to be rendered at a screen position.
pub struct TextSection {
    /// Top-left corner in screen pixels (y down).
    pub x: f32,
    pub y: f32,
    pub text: String,
    /// Rendered line-height in screen pixels.  The atlas is baked at
    /// `RASTER_PX`; other sizes are scaled at the vertex level.
    pub size: f32,
    pub color: [f32; 4],
}

// ── Internal types ────────────────────────────────────────────────────────────

/// Per-glyph atlas metadata (all in atlas-pixel coordinates).
#[derive(Clone, Copy, Default)]
pub(super) struct Glyph {
    pub cell_x: u32,
    pub cell_y: u32,
    pub bmp_w: u32,
    pub bmp_h: u32,
    pub bear_x: f32,
    pub bear_y: f32,
    pub advance: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(super) struct Vertex {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

// ── Shader ───────────────────────────────────────────────────────────────────

pub(super) const SHADER: &str = r#"
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
