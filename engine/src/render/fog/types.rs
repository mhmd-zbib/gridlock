use bytemuck::{Pod, Zeroable};

pub const MAX_FOG_SOURCES: usize = 8;

/// One light source for the fog pass.
///
/// The stencil buffer (ray-cast, wall-aware) provides hard wall edges.
/// This struct drives the gradient shader: radial falloff for the halo/beam
/// and angular feathering for soft cone edges in open air.
/// All positional/distance values are in **screen pixels**.
///
/// Layout (32 bytes, stride 32):
///   pos            vec2<f32>  offset  0
///   cone_range     f32        offset  8
///   circle_radius  f32        offset 12
///   direction      vec2<f32>  offset 16
///   half_angle     f32        offset 24
///   _pad           f32        offset 28
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct FogSource {
    /// Screen-pixel position of the cone origin.
    pub pos: [f32; 2],
    /// Maximum range of the directional cone in screen pixels.
    pub cone_range: f32,
    /// Radius of the omnidirectional near-halo in screen pixels.
    pub circle_radius: f32,
    /// Unit vector pointing in the cone's facing direction (screen space).
    pub direction: [f32; 2],
    /// Half-angle of the cone in radians. 0 means no directional cone.
    pub half_angle: f32,
    pub _pad: f32,
}

impl FogSource {
    pub fn new(
        pos: [f32; 2],
        cone_range_px: f32,
        circle_radius_px: f32,
        direction: [f32; 2],
        half_angle: f32,
    ) -> Self {
        Self {
            pos,
            cone_range: cone_range_px,
            circle_radius: circle_radius_px,
            direction,
            half_angle,
            _pad: 0.0,
        }
    }
}

/// Full uniform block uploaded to the GPU each frame.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct FogUniforms {
    pub screen_size: [f32; 2],
    /// Alpha of the black overlay applied outside the visible cone.
    pub outside_dim: f32,
    pub source_count: u32,
    pub sources: [FogSource; MAX_FOG_SOURCES],
}

impl FogUniforms {
    pub fn new(screen_size: [f32; 2], outside_dim: f32) -> Self {
        Self {
            screen_size,
            outside_dim,
            source_count: 0,
            sources: [FogSource::zeroed(); MAX_FOG_SOURCES],
        }
    }

    pub fn push(&mut self, source: FogSource) {
        let idx = self.source_count as usize;
        if idx < MAX_FOG_SOURCES {
            self.sources[idx] = source;
            self.source_count += 1;
        }
    }
}
