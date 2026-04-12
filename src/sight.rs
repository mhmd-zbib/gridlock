use crate::wall::Wall;

// ---------------------------------------------------------------------------
// Sight
// ---------------------------------------------------------------------------

pub struct Sight {
    /// Current facing direction in radians (0 = right, PI/2 = down).
    pub direction:     f32,
    /// Half-angle of the vision cone in radians.
    pub half_angle:    f32,
    /// How far the cone reaches in pixels.
    pub range:         f32,
    /// Radius of the always-visible bubble around the entity.
    pub circle_radius: f32,
}

impl Sight {
    pub fn player() -> Self {
        Self {
            direction:     0.0,
            half_angle:    40_f32.to_radians(),
            range:         320.0,
            circle_radius: 80.0,
        }
    }

    pub fn enemy() -> Self {
        Self {
            direction:     std::f32::consts::PI,
            half_angle:    38_f32.to_radians(),
            range:         260.0,
            circle_radius: 60.0,
        }
    }

    /// Rotate to face `to` from `from`.  No-op if the points are the same.
    pub fn face(&mut self, from: (f32, f32), to: (f32, f32)) {
        let dx = to.0 - from.0;
        let dy = to.1 - from.1;
        if dx * dx + dy * dy > 0.01 {
            self.direction = dy.atan2(dx);
        }
    }

    /// Returns `true` if `target` is visible from `from`:
    /// - within the nearby circle (+ unobstructed line of sight), OR
    /// - inside the cone and unobstructed.
    pub fn can_see(&self, from: (f32, f32), target: (f32, f32), walls: &[Wall]) -> bool {
        let dx   = target.0 - from.0;
        let dy   = target.1 - from.1;
        let dist = (dx * dx + dy * dy).sqrt();

        // Circle bubble — always visible if nearby and unobstructed.
        if dist <= self.circle_radius {
            return has_los(from, target, walls);
        }

        // Cone — must be in range, in angle, and unobstructed.
        if dist > self.range { return false; }

        let angle_to = dy.atan2(dx);
        if wrap_angle(angle_to - self.direction).abs() > self.half_angle {
            return false;
        }

        has_los(from, target, walls)
    }

    /// Cast `n_rays` evenly across the cone and return the arc endpoint
    /// of each ray (wall-clipped).  The first element is at
    /// `direction - half_angle`, the last at `direction + half_angle`.
    pub fn cone_arc_pts(&self, origin: (f32, f32), walls: &[Wall], n_rays: usize) -> Vec<[f32; 2]> {
        (0..=n_rays)
            .map(|i| {
                let t     = i as f32 / n_rays as f32;
                let angle = self.direction - self.half_angle + t * self.half_angle * 2.0;
                let dir   = (angle.cos(), angle.sin());
                let dist  = cast_ray(origin, dir, self.range, walls);
                [origin.0 + dir.0 * dist, origin.1 + dir.1 * dist]
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Wrap an angle difference into (-π, π].
fn wrap_angle(mut d: f32) -> f32 {
    use std::f32::consts::TAU;
    while d >  std::f32::consts::PI { d -= TAU; }
    while d < -std::f32::consts::PI { d += TAU; }
    d
}

/// Returns `true` if the straight line from `from` to `to` is unobstructed.
fn has_los(from: (f32, f32), to: (f32, f32), walls: &[Wall]) -> bool {
    let dx  = to.0 - from.0;
    let dy  = to.1 - from.1;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 0.001 { return true; }
    let dir = (dx / len, dy / len);
    walls.iter().all(|w| {
        ray_aabb(from, dir, w).map_or(true, |t| t <= 0.5 || t >= len)
    })
}

/// Walk a ray and return the distance to the first wall hit (or `max_dist`).
fn cast_ray(origin: (f32, f32), dir: (f32, f32), max_dist: f32, walls: &[Wall]) -> f32 {
    walls.iter()
        .filter_map(|w| ray_aabb(origin, dir, w))
        .filter(|&t| t > 0.5)
        .fold(max_dist, f32::min)
}

/// Ray–AABB intersection using the slab method.
/// Returns the entry distance along `dir`, or `None` if no hit.
fn ray_aabb(origin: (f32, f32), dir: (f32, f32), wall: &Wall) -> Option<f32> {
    let inv_x = if dir.0.abs() > 1e-9 { 1.0 / dir.0 } else { f32::INFINITY };
    let inv_y = if dir.1.abs() > 1e-9 { 1.0 / dir.1 } else { f32::INFINITY };

    let tx1 = (wall.x           - origin.0) * inv_x;
    let tx2 = (wall.x + wall.w  - origin.0) * inv_x;
    let ty1 = (wall.y           - origin.1) * inv_y;
    let ty2 = (wall.y + wall.h  - origin.1) * inv_y;

    let tmin = tx1.min(tx2).max(ty1.min(ty2));
    let tmax = tx1.max(tx2).min(ty1.max(ty2));

    if tmax >= tmin && tmin > 0.0 { Some(tmin) } else { None }
}
