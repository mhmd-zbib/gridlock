
// ---------------------------------------------------------------------------
// Tunable constants — edit these to feel the behaviour
// ---------------------------------------------------------------------------

/// Half-angle of the spread cone when perfectly still and not shooting (degrees).
/// Full base cone width = 2 × BASE_HALF_ANGLE_DEG.
pub const DEFAULT_BASE_HALF_ANGLE_DEG: f32 = 1.5;

/// Maximum extra half-angle added when moving at full speed (degrees).
/// Formula: movement_spread = MOVEMENT_SPREAD_MAX_DEG × smoothed_speed^MOVEMENT_SPREAD_POWER
pub const DEFAULT_MOVEMENT_SPREAD_MAX_DEG: f32 = 12.0;

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
    base_half_angle_deg: f32,
    movement_spread_max_deg: f32,

    /// Tiny embedded xorshift32 PRNG — no external crate needed.
    rng_state: u32,
}

impl AimCone {
    pub fn new() -> Self {
        Self {
            direction: 0.0,
            recoil_spread: 0.0,
            smoothed_velocity_frac: 0.0,
            base_half_angle_deg: DEFAULT_BASE_HALF_ANGLE_DEG,
            movement_spread_max_deg: DEFAULT_MOVEMENT_SPREAD_MAX_DEG,
            rng_state: 0xDEAD_BEEF,
        }
    }

    pub fn set_spread_profile(&mut self, base_half_angle_deg: f32, movement_spread_max_deg: f32) {
        self.base_half_angle_deg = base_half_angle_deg.max(0.0);
        self.movement_spread_max_deg = movement_spread_max_deg.max(0.0);
    }

    /// Reconstruct `recoil_spread` from a server-authoritative half-angle so the
    /// local aim cone state agrees with what the server actually computed.
    /// Called during reconciliation so bullet sampling and the rendered cone use
    /// the same spread.
    pub fn sync_from_server_half_angle(&mut self, server_half_angle: f32) {
        let base = self.base_half_angle_deg.to_radians();
        let movement = self.movement_spread_max_deg.to_radians()
            * self.smoothed_velocity_frac.powf(MOVEMENT_SPREAD_POWER);
        self.recoil_spread = (server_half_angle - base - movement).max(0.0);
    }

    // -----------------------------------------------------------------------
    // State mutators
    // -----------------------------------------------------------------------

    /// Call once per bullet fired to widen the spread cone.
    pub fn on_shot(&mut self, recoil_per_shot_deg: f32, recoil_max_deg: f32) {
        let per_shot = recoil_per_shot_deg.max(0.0).to_radians();
        let max = recoil_max_deg.max(0.0).to_radians();
        self.recoil_spread = (self.recoil_spread + per_shot).min(max);
    }

    /// Advance the cone simulation one frame.
    ///
    /// `velocity_frac` is the raw speed fraction from movement (0 = still, 1 = full speed).
    /// The cone smooths it internally — callers just pass the instantaneous value.
    pub fn update(&mut self, dt: f32, velocity_frac: f32, recoil_decay_deg_per_sec: f32) {
        // Recoil decays at a fixed rate regardless of whether we're shooting.
        let decay = recoil_decay_deg_per_sec.max(0.0).to_radians() * dt;
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
        let movement_spread = self.movement_spread_max_deg.to_radians()
            * self.smoothed_velocity_frac.powf(MOVEMENT_SPREAD_POWER);
        self.base_half_angle_deg.to_radians() + self.recoil_spread + movement_spread
    }

    /// Sample a random direction vector for a fired bullet, spread within the current cone.
    /// Uses the internal stateful PRNG — suitable for singleplayer / AI enemies.
    pub fn sample_direction(&mut self) -> (f32, f32) {
        let half = self.half_angle();
        let offset = (self.next_f32() * 2.0 - 1.0) * half;
        let angle = self.direction + offset;
        (angle.cos(), angle.sin())
    }

    /// Pure, deterministic variant driven by an external `seed`.
    ///
    /// Both client and server call this with the same seed so shot prediction
    /// and server validation agree.  The bullet direction is uniform across the
    /// full cone width — every angle in `[-half, +half]` is equally reachable
    /// so the cone boundary is always the hard limit for where bullets can land.
    pub fn sample_direction_seeded(&self, seed: u32) -> (f32, f32) {
        let half = self.half_angle();
        // Uniform offset in [-half, +half]: the bullet angle is always exactly
        // within the cone and can reach both edges (r≈0 → -half, r≈1 → +half).
        // Angle addition is the only correct approach — the vector formula
        // normalize(d + δ·u) produces atan(δ) deviation, not δ, so it
        // under-spreads at large half-angles (movement spread, recoil buildup).
        let r = xorshift32(seed) as f32 / u32::MAX as f32;
        let angle = self.direction + (r * 2.0 - 1.0) * half;
        (angle.cos(), angle.sin())
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

/// One step of xorshift32 — a fast, portable, deterministic PRNG with no deps.
/// Used by `AimCone::sample_direction_seeded` so client and server agree on spread.
pub fn xorshift32(mut state: u32) -> u32 {
    state ^= state << 13;
    state ^= state >> 17;
    state ^= state << 5;
    state
}
