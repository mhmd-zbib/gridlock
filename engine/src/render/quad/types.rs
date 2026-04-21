use bytemuck::{Pod, Zeroable};

/// One colored rectangle in screen space.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct QuadInstance {
    pub center: [f32; 2],
    pub half_size: [f32; 2],
    pub color: [f32; 4],
}

/// One vertically-gradient rectangle in screen space.
///
/// The shader interpolates from `bottom_color` at the bottom edge to
/// `top_color` at the top edge.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct GradientQuadInstance {
    pub center: [f32; 2],
    pub half_size: [f32; 2],
    pub top_color: [f32; 4],
    pub bottom_color: [f32; 4],
}

/// One shaded UI rectangle in screen space.
///
/// Colors and shape parameters are consumed by the shaded UI fragment shader.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct ShadedQuadInstance {
    pub center: [f32; 2],
    pub half_size: [f32; 2],
    pub top_color: [f32; 4],
    pub bottom_color: [f32; 4],
    pub stroke_color: [f32; 4],
    pub accent_color: [f32; 4],
    /// x = border width fraction, y = corner-cut fraction,
    /// z = accent width fraction, w = reserved.
    pub params: [f32; 4],
}
