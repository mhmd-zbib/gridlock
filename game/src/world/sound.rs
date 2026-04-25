use ecs::Component;

use crate::world::units::px_to_tiles;

const R_MIN: f32 = px_to_tiles(20.0);
const R_MAX: f32 = px_to_tiles(180.0);
const S_MAX: f32 = px_to_tiles(200.0);
const ALPHA: f32 = 0.15;
const GAMMA: f32 = 1.4;
const PULSE_C: f32 = px_to_tiles(90.0);
const PULSE_LAMBDA: f32 = 4.0;
const PULSE_A0: f32 = 1.0;
const PULSE_EPSILON: f32 = 0.02;
const STEP_INTERVAL: f32 = 0.32;
const MAX_PULSES: usize = 6;
const MOVE_THRESHOLD: f32 = px_to_tiles(5.0);

// ---------------------------------------------------------------------------
// Pulse
// ---------------------------------------------------------------------------

/// A single expanding footstep ring emitted when the player moves.
pub struct Pulse {
    pub age: f32,
}

impl Pulse {
    /// Current radius in tile units.
    pub fn radius(&self) -> f32 {
        PULSE_C * self.age
    }

    /// Current amplitude in [0, 1].
    pub fn amplitude(&self) -> f32 {
        PULSE_A0 * (-PULSE_LAMBDA * self.age).exp()
    }

    fn is_alive(&self) -> bool {
        self.amplitude() > PULSE_EPSILON
    }
}

// ---------------------------------------------------------------------------
// SoundField
// ---------------------------------------------------------------------------

/// Speed-driven audible radius with exponential smoothing and pulse rings.
///
/// Call `tick` every game step.  Read `smoothed_radius` for AI detection and
/// `pulses` for rendering expanding footstep rings.
pub struct SoundField {
    /// Exponentially smoothed radius in tile units — use for AI threshold checks.
    pub smoothed_radius: f32,
    /// Active expanding rings (newest last).
    pub pulses: Vec<Pulse>,
    step_accum: f32,
}

impl Default for SoundField {
    fn default() -> Self {
        Self::new()
    }
}

impl SoundField {
    pub fn new() -> Self {
        Self {
            smoothed_radius: R_MIN,
            pulses: Vec::new(),
            step_accum: 0.0,
        }
    }

    /// Advance the sound field by `dt` seconds at the given world speed (tiles/s).
    pub fn tick(&mut self, speed: f32, dt: f32) {
        let speed_norm = (speed / S_MAX).clamp(0.0, 1.0);
        let base = R_MIN + (R_MAX - R_MIN) * speed_norm.powf(GAMMA);
        self.smoothed_radius += (base - self.smoothed_radius) * ALPHA;

        for pulse in &mut self.pulses {
            pulse.age += dt;
        }
        self.pulses.retain(|p| p.is_alive());

        if speed > MOVE_THRESHOLD {
            self.step_accum += dt;
            if self.step_accum >= STEP_INTERVAL {
                self.step_accum -= STEP_INTERVAL;
                if self.pulses.len() < MAX_PULSES {
                    self.pulses.push(Pulse { age: 0.0 });
                }
            }
        } else {
            self.step_accum = 0.0;
        }
    }

    /// Returns `true` if a `listener` can hear this source based on the smoothed radius.
    pub fn hears(&self, source: (f32, f32), listener: (f32, f32)) -> bool {
        let dx = listener.0 - source.0;
        let dy = listener.1 - source.1;
        dx * dx + dy * dy <= self.smoothed_radius * self.smoothed_radius
    }
}

impl Component for SoundField {}
