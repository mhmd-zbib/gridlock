use bytemuck::{Pod, Zeroable};

/// One colored rectangle in screen space.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct QuadInstance {
    pub center: [f32; 2],
    pub half_size: [f32; 2],
    pub color: [f32; 4],
}

/// Per-frame visibility parameters passed to the scene/enemy draw calls.
#[derive(Clone, Copy)]
pub struct VisParams {
    pub player_pos: [f32; 2],
    pub player_dir: [f32; 2],
    pub cos_half_fov: f32,
    pub cone_range: f32,
    pub circle_radius: f32,
    pub ambient: f32,
}
