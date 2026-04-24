pub const MAX_LIGHTS: usize = 16;

/// Sentinel value for `cos_half_angle` that disables cone attenuation.
/// Any value below -1.0 works; -2.0 is unambiguous.
pub const NO_CONE: f32 = -2.0;

/// A single point or cone light described in framebuffer/screen space.
///
/// All distances are in pixels with origin at the top-left of the window.
///
/// GPU layout (WGSL-compatible, 48 bytes, stride 48):
/// ```text
/// offset  0  screen_pos      vec2<f32>   (align 8)
/// offset  8  radius          f32
/// offset 12  intensity       f32
/// offset 16  color           vec3<f32>   (align 16)
/// offset 28  cos_half_angle  f32
/// offset 32  direction       vec2<f32>   (align 8)
/// offset 40  _pad            vec2<f32>
/// total: 48 bytes
/// ```
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuLight {
    /// Center position in framebuffer pixels.
    pub screen_pos: [f32; 2],
    /// Radius of influence in pixels — light falls off to zero at this distance.
    pub radius: f32,
    /// Intensity multiplier applied on top of the RGB color.
    pub intensity: f32,
    /// Linear RGB color of the light.
    pub color: [f32; 3],
    /// cos(half_angle) for cone attenuation.
    /// Use [`NO_CONE`] (-2.0) to disable cone and emit as a full point light.
    pub cos_half_angle: f32,
    /// Normalized direction vector in screen space.  Ignored when cos_half_angle == NO_CONE.
    pub direction: [f32; 2],
    pub _pad: [f32; 2],
}

impl GpuLight {
    /// Create a full-circle point light (no directional cone).
    pub fn point(screen_pos: [f32; 2], radius: f32, intensity: f32, color: [f32; 3]) -> Self {
        Self {
            screen_pos,
            radius,
            intensity,
            color,
            cos_half_angle: NO_CONE,
            direction: [0.0; 2],
            _pad: [0.0; 2],
        }
    }

    /// Create a directional cone light.
    ///
    /// `direction` must be a **normalised** screen-space direction vector.
    /// `cos_half_angle` is the cosine of the cone's half-angle
    /// (e.g. `(π/4_f32).cos()` for a 90° cone).
    pub fn cone(
        screen_pos: [f32; 2],
        radius: f32,
        intensity: f32,
        color: [f32; 3],
        direction: [f32; 2],
        cos_half_angle: f32,
    ) -> Self {
        Self {
            screen_pos,
            radius,
            intensity,
            color,
            cos_half_angle,
            direction,
            _pad: [0.0; 2],
        }
    }
}

/// Per-frame lighting data uploaded to the GPU.
///
/// The ambient term is applied unconditionally to every lit texel.  Each point
/// light adds a smooth-falloff radial contribution.  When `count == 0` and
/// `ambient == [1.0; 4]` the result is identical to unlit rendering.
///
/// GPU layout (WGSL-compatible, 800 bytes):
/// ```text
/// offset  0  ambient   vec4<f32>
/// offset 16  count     u32
/// offset 20  _pad      [u32; 3]
/// offset 32  lights    array<GpuLight, 16>   (16 × 48 = 768 bytes)
/// total: 800 bytes
/// ```
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightingUniform {
    /// Minimum light level (RGB; alpha unused).
    pub ambient: [f32; 4],
    /// Number of active lights in `lights`; the rest are ignored.
    pub count: u32,
    pub _pad: [u32; 3],
    pub lights: [GpuLight; MAX_LIGHTS],
}

impl Default for LightingUniform {
    fn default() -> Self {
        Self {
            ambient: [1.0, 1.0, 1.0, 1.0],
            count: 0,
            _pad: [0; 3],
            lights: [GpuLight {
                screen_pos: [0.0; 2],
                radius: 0.0,
                intensity: 0.0,
                color: [0.0; 3],
                cos_half_angle: NO_CONE,
                direction: [0.0; 2],
                _pad: [0.0; 2],
            }; MAX_LIGHTS],
        }
    }
}

impl LightingUniform {
    /// Add a light, clamping silently once `MAX_LIGHTS` is reached.
    pub fn push(&mut self, light: GpuLight) {
        let idx = self.count as usize;
        if idx < MAX_LIGHTS {
            self.lights[idx] = light;
            self.count += 1;
        }
    }
}
