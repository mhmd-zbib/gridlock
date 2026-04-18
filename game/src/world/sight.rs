use super::ray::{has_los, wrap_angle};
use super::units::px_to_tiles;
use super::wall::Wall;

// ---------------------------------------------------------------------------
// Sight
// ---------------------------------------------------------------------------

pub struct Sight {
    /// Current facing direction in radians (0 = right, PI/2 = down).
    pub direction: f32,
    /// Half-angle of the vision cone in radians.
    pub half_angle: f32,
    /// How far the cone reaches in tiles.
    pub range: f32,
    /// Radius of the always-visible bubble around the entity.
    pub circle_radius: f32,
    /// Maximum rotation speed in radians per second.
    pub turn_speed: f32,
}

impl Sight {
    pub fn player() -> Self {
        Self {
            direction: 0.0,
            half_angle: 40_f32.to_radians(),
            range: px_to_tiles(320.0),
            circle_radius: px_to_tiles(80.0),
            turn_speed: 12.0,
        }
    }

    pub fn enemy() -> Self {
        Self {
            direction: std::f32::consts::PI,
            half_angle: 38_f32.to_radians(),
            range: px_to_tiles(260.0),
            circle_radius: px_to_tiles(60.0),
            turn_speed: 6.0,
        }
    }

    /// Smoothly rotate to face `to` from `from` using `turn_speed`.
    pub fn face(&mut self, from: (f32, f32), to: (f32, f32), dt: f32) {
        let dx = to.0 - from.0;
        let dy = to.1 - from.1;
        if dx * dx + dy * dy > 0.01 {
            let target = dy.atan2(dx);
            let diff = wrap_angle(target - self.direction);
            let step = self.turn_speed * dt;
            if diff.abs() <= step {
                self.direction = target;
            } else {
                self.direction += step * diff.signum();
            }
        }
    }

    /// Returns `true` if `target` is visible from `from`:
    /// - within the nearby circle (+ unobstructed line of sight), OR
    /// - inside the cone and unobstructed.
    pub fn can_see(&self, from: (f32, f32), target: (f32, f32), walls: &[Wall]) -> bool {
        let dx = target.0 - from.0;
        let dy = target.1 - from.1;
        let dist = (dx * dx + dy * dy).sqrt();

        // Circle bubble — always visible if nearby and unobstructed.
        if dist <= self.circle_radius {
            return has_los(from, target, walls);
        }

        // Cone — must be in range, in angle, and unobstructed.
        if dist > self.range {
            return false;
        }

        let angle_to = dy.atan2(dx);
        if wrap_angle(angle_to - self.direction).abs() > self.half_angle {
            return false;
        }

        has_los(from, target, walls)
    }
}
