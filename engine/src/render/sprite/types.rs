use bytemuck::{Pod, Zeroable};

use crate::asset::AssetHandle;

/// A single textured-quad draw request submitted by the game layer.
///
/// Commands are collected each frame, depth-sorted (z ascending = back to
/// front), batched by `handle`, and issued as one GPU draw call per batch.
pub struct SpriteCommand {
    /// GPU texture to sample.
    pub handle: AssetHandle,
    /// Screen-space centre (pixels from top-left).
    pub pos: [f32; 2],
    /// Half-extents of the quad in pixels.
    pub half_size: [f32; 2],
    /// Clockwise rotation in radians.
    pub rotation: f32,
    /// Painter's depth: lower values are drawn first (further back).
    pub z: f32,
    /// UV sub-rectangle [u_min, v_min, u_max, v_max].
    /// Use `[0.0, 0.0, 1.0, 1.0]` for the full texture.
    pub uv_rect: [f32; 4],
    /// RGBA colour multiplied with the sampled texture colour.
    pub tint: [f32; 4],
}

impl SpriteCommand {
    /// Convenience constructor for an unrotated, full-texture, opaque sprite.
    pub fn simple(handle: AssetHandle, pos: [f32; 2], half_size: [f32; 2], z: f32) -> Self {
        Self {
            handle,
            pos,
            half_size,
            rotation: 0.0,
            z,
            uv_rect: [0.0, 0.0, 1.0, 1.0],
            tint: [1.0, 1.0, 1.0, 1.0],
        }
    }
}

// ---------------------------------------------------------------------------
// GPU instance layout — must stay in sync with the shader's InstIn struct.
// ---------------------------------------------------------------------------

/// Per-instance vertex data uploaded to the GPU each frame.
///
/// Field order matches the `vertex_attr_array!` declaration in the renderer
/// (locations 1–6).  Total size: 52 bytes; no hidden padding with repr(C).
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub(super) struct SpriteInstance {
    pub center:    [f32; 2], // location 1 — offset  0
    pub half_size: [f32; 2], // location 2 — offset  8
    pub rotation:  f32,      // location 3 — offset 16
    pub uv_min:    [f32; 2], // location 4 — offset 20
    pub uv_max:    [f32; 2], // location 5 — offset 28
    pub tint:      [f32; 4], // location 6 — offset 36
}                            //              total  52
