use super::ray::{cast_ray, wrap_angle};
use super::wall::Wall;

// ---------------------------------------------------------------------------
// Tunable constants — edit these to feel the behaviour
// ---------------------------------------------------------------------------

/// Half-angle of the spread cone when perfectly still and not shooting (degrees).
/// Full base cone width = 2 × BASE_HALF_ANGLE_DEG.
pub const BASE_HALF_ANGLE_DEG: f32 = 1.5;

/// Degrees added to recoil half-angle on each bullet fired.
pub const RECOIL_PER_SHOT_DEG: f32 = 4.0;

/// Maximum recoil half-angle the cone will ever reach (degrees).
/// Cone can still be widened further by movement.
pub const RECOIL_MAX_DEG: f32 = 20.0;

/// How fast the recoil half-angle shrinks back to zero when not firing (degrees/second).
pub const RECOIL_DECAY_DEG_PER_SEC: f32 = 10.0;

/// Maximum extra half-angle added when moving at full speed (degrees).
/// Formula: movement_spread = MOVEMENT_SPREAD_MAX_DEG × smoothed_speed^MOVEMENT_SPREAD_POWER
pub const MOVEMENT_SPREAD_MAX_DEG: f32 = 12.0;

/// Exponent for the movement-spread curve.
///   1.0 = linear (even spread across all speeds)
///   2.0 = squared (spread ramps up faster near top speed)
pub const MOVEMENT_SPREAD_POWER: f32 = 1.5;

/// How fast the smoothed speed fraction rises when the player starts moving (units/second).
/// 3.0 means the cone reaches full movement spread in ~0.33 s after starting to walk.
pub const MOVEMENT_SPREAD_RISE_RATE: f32 = 3.0;

/// How fast the smoothed speed fraction falls when the player stops moving (units/second).
/// Lower than rise so the cone lingers a moment after stopping — feels more physical.
pub const MOVEMENT_SPREAD_FALL_RATE: f32 = 1.8;

/// How far the aim-cone arc is drawn in pixels (visual only, not a bullet range cap).
pub const AIM_CONE_RENDER_RANGE: f32 = 180.0;

// ---------------------------------------------------------------------------
// AimCone
// ---------------------------------------------------------------------------

pub struct AimCone {
    /// Current aiming direction in radians (matches Sight::direction).
    pub direction: f32,

    /// Accumulated recoil spread half-angle in radians.
    /// Grows on each shot, decays when not shooting.
    recoil_spread: f32,

    /// Smoothed version of the raw velocity fraction (0 = still, 1 = full speed).
    /// Ramps toward the real value at different rates for rising vs falling,
    /// so the cone grows and shrinks gradually instead of snapping.
    smoothed_velocity_frac: f32,

    /// Tiny embedded xorshift32 PRNG — no external crate needed.
    rng_state: u32,
}

impl AimCone {
    pub fn new() -> Self {
        Self {
            direction: 0.0,
            recoil_spread: 0.0,
            smoothed_velocity_frac: 0.0,
            rng_state: 0xDEAD_BEEF,
        }
    }

    // -----------------------------------------------------------------------
    // State mutators
    // -----------------------------------------------------------------------

    /// Call once per bullet fired to widen the spread cone.
    pub fn on_shot(&mut self) {
        let max = RECOIL_MAX_DEG.to_radians();
        self.recoil_spread = (self.recoil_spread + RECOIL_PER_SHOT_DEG.to_radians()).min(max);
    }

    /// Advance the cone simulation one frame.
    ///
    /// `velocity_frac` is the raw speed fraction from movement (0 = still, 1 = full speed).
    /// The cone smooths it internally — callers just pass the instantaneous value.
    pub fn update(&mut self, dt: f32, velocity_frac: f32) {
        // Recoil decays at a fixed rate regardless of whether we're shooting.
        let decay = RECOIL_DECAY_DEG_PER_SEC.to_radians() * dt;
        self.recoil_spread = (self.recoil_spread - decay).max(0.0);

        // Smooth the velocity fraction: use a faster rate when rising (player starts
        // moving) and a slower rate when falling (player stops moving).  This gives the
        // cone a slight lag in both directions — it builds up with some inertia and
        // lingers briefly after the player stops, which reads as physically plausible.
        let rate = if velocity_frac > self.smoothed_velocity_frac {
            MOVEMENT_SPREAD_RISE_RATE
        } else {
            MOVEMENT_SPREAD_FALL_RATE
        };
        let delta = velocity_frac - self.smoothed_velocity_frac;
        self.smoothed_velocity_frac += delta.signum() * (rate * dt).min(delta.abs());
    }

    // -----------------------------------------------------------------------
    // Queries
    // -----------------------------------------------------------------------

    /// Total current half-angle in radians (base + recoil + smoothed movement spread).
    pub fn half_angle(&self) -> f32 {
        let movement_spread = MOVEMENT_SPREAD_MAX_DEG.to_radians()
            * self.smoothed_velocity_frac.powf(MOVEMENT_SPREAD_POWER);
        BASE_HALF_ANGLE_DEG.to_radians() + self.recoil_spread + movement_spread
    }

    /// Sample a random direction vector for a fired bullet, spread within the current cone.
    pub fn sample_direction(&mut self) -> (f32, f32) {
        let half = self.half_angle();
        let offset = (self.next_f32() * 2.0 - 1.0) * half;
        let angle = self.direction + offset;
        (angle.cos(), angle.sin())
    }

    // -----------------------------------------------------------------------
    // Rendering
    // -----------------------------------------------------------------------

    /// Cast rays across the aim cone and return wall-clipped arc points for rendering.
    ///
    /// The algorithm is identical to `Sight::cone_arc_pts`: uniform ray samples are
    /// supplemented with wall-corner rays so shadow edges snap cleanly to geometry.
    pub fn cone_arc_pts(&self, origin: (f32, f32), walls: &[Wall], n_rays: usize) -> Vec<[f32; 2]> {
        let half = self.half_angle();
        let range = AIM_CONE_RENDER_RANGE;

        let mut rel: Vec<f32> = (0..=n_rays)
            .map(|i| {
                let t = i as f32 / n_rays as f32;
                -half + t * half * 2.0
            })
            .collect();

        // Corner-angle hints — same trick as Sight — keeps edges crisp.
        const EPS: f32 = 0.0002;
        for w in walls {
            for &(cx, cy) in &[
                (w.x, w.y),
                (w.x + w.w, w.y),
                (w.x, w.y + w.h),
                (w.x + w.w, w.y + w.h),
            ] {
                let dx = cx - origin.0;
                let dy = cy - origin.1;
                if dx * dx + dy * dy < 1.0 {
                    continue;
                }
                let r = wrap_angle(dy.atan2(dx) - self.direction);
                if r.abs() < half {
                    rel.push(r - EPS);
                    rel.push(r);
                    rel.push(r + EPS);
                }
            }
        }

        rel.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        rel.iter()
            .map(|&r| {
                let angle = self.direction + r.clamp(-half, half);
                let dir = (angle.cos(), angle.sin());
                let dist = cast_ray(origin, dir, range, walls);
                [origin.0 + dir.0 * dist, origin.1 + dir.1 * dist]
            })
            .collect()
    }

    // -----------------------------------------------------------------------
    // PRNG (xorshift32 — no external crate)
    // -----------------------------------------------------------------------

    /// Returns a pseudo-random f32 in [0, 1).
    fn next_f32(&mut self) -> f32 {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 17;
        self.rng_state ^= self.rng_state << 5;
        self.rng_state as f32 / u32::MAX as f32
    }
}
