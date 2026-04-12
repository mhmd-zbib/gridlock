use super::ray::{cast_ray, has_los, wrap_angle};
use super::wall::Wall;

// ---------------------------------------------------------------------------
// Sight
// ---------------------------------------------------------------------------

pub struct Sight {
    /// Current facing direction in radians (0 = right, PI/2 = down).
    pub direction: f32,
    /// Half-angle of the vision cone in radians.
    pub half_angle: f32,
    /// How far the cone reaches in pixels.
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
            range: 320.0,
            circle_radius: 80.0,
            turn_speed: 12.0,
        }
    }

    pub fn enemy() -> Self {
        Self {
            direction: std::f32::consts::PI,
            half_angle: 38_f32.to_radians(),
            range: 260.0,
            circle_radius: 60.0,
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

    /// Cast rays around the nearby circle and return the clipped outline points.
    ///
    /// This uses the same wall-corner hinting as the cone so nearby-vision edges
    /// snap cleanly to geometry while moving.
    pub fn circle_arc_pts(
        &self,
        origin: (f32, f32),
        walls: &[Wall],
        n_rays: usize,
    ) -> Vec<[f32; 2]> {
        use std::f32::consts::{PI, TAU};

        let n = n_rays.max(8);
        let mut angles: Vec<f32> = (0..n)
            .map(|i| -PI + (i as f32 / n as f32) * TAU)
            .collect();

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
                let a = dy.atan2(dx);
                angles.push(wrap_angle(a - EPS));
                angles.push(a);
                angles.push(wrap_angle(a + EPS));
            }
        }

        angles.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let mut pts: Vec<[f32; 2]> = angles
            .iter()
            .map(|&a| {
                let dir = (a.cos(), a.sin());
                let dist = cast_ray(origin, dir, self.circle_radius, walls);
                [origin.0 + dir.0 * dist, origin.1 + dir.1 * dist]
            })
            .collect();

        if let Some(first) = pts.first().copied() {
            pts.push(first);
        }

        pts
    }

    /// Cast rays across the cone and return the arc endpoints (wall-clipped).
    ///
    /// Uniform samples are supplemented with rays aimed at each wall corner
    /// (± a tiny epsilon) so shadow edges always snap to geometry and don't
    /// jiggle as the player moves.
    pub fn cone_arc_pts(&self, origin: (f32, f32), walls: &[Wall], n_rays: usize) -> Vec<[f32; 2]> {
        // Work in relative-angle space so wrapping is handled cleanly.
        let mut rel: Vec<f32> = (0..=n_rays)
            .map(|i| {
                let t = i as f32 / n_rays as f32;
                -self.half_angle + t * self.half_angle * 2.0
            })
            .collect();

        // Add corner angles for every wall that falls inside the cone.
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
                if r.abs() < self.half_angle {
                    rel.push(r - EPS);
                    rel.push(r);
                    rel.push(r + EPS);
                }
            }
        }

        rel.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        rel.iter()
            .map(|&r| {
                let angle = self.direction + r.clamp(-self.half_angle, self.half_angle);
                let dir = (angle.cos(), angle.sin());
                let dist = cast_ray(origin, dir, self.range, walls);
                [origin.0 + dir.0 * dist, origin.1 + dir.1 * dist]
            })
            .collect()
    }
}
