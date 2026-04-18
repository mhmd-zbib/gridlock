use bytemuck::{Pod, Zeroable};

/// One colored rectangle in screen space.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct QuadInstance {
    pub center: [f32; 2],
    pub half_size: [f32; 2],
    pub color: [f32; 4],
}
