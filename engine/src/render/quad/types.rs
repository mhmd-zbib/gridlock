use bytemuck::{Pod, Zeroable};

/// Per-enemy quad with embedded visibility cone parameters for per-pixel soft reveal.
///
/// The fragment shader computes V(P) = A(P)·D(P) + halo per pixel and uses it
/// as an alpha multiplier so cone edges fade smoothly rather than clipping hard.
/// The stencil (wall-aware ray-cast polygon) still provides hard wall occlusion.
///
/// Layout (80 bytes, stride 80):
///   center       vec2<f32>  offset  0
///   half_size    vec2<f32>  offset  8
///   color        vec4<f32>  offset 16  (a = temporal vis_alpha)
///   viewer_pos   vec2<f32>  offset 32
///   view_dir     vec2<f32>  offset 40
///   cone_a       vec4<f32>  offset 48  [cos_inner, cos_outer, range_inner_px, range_outer_px]
///   cone_b       vec4<f32>  offset 64  [circle_radius_px, pad, pad, pad]
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct ConeVisInstance {
    pub center: [f32; 2],
    pub half_size: [f32; 2],
    pub color: [f32; 4],
    pub viewer_pos: [f32; 2],
    pub view_dir: [f32; 2],
    /// [cos_inner, cos_outer, range_inner_px, range_outer_px]
    pub cone_a: [f32; 4],
    /// [circle_radius_px, _pad, _pad, _pad]
    pub cone_b: [f32; 4],
}

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
