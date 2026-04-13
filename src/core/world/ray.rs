use super::units::px_to_tiles;
use super::wall::Wall;

// ---------------------------------------------------------------------------
// Shared ray-casting utilities
// Used by both Sight (visibility) and AimCone (bullet spread).
// ---------------------------------------------------------------------------

/// Wrap an angle difference into (-π, π].
pub fn wrap_angle(mut d: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    while d > PI {
        d -= TAU;
    }
    while d < -PI {
        d += TAU;
    }
    d
}

/// Returns `true` if the straight line from `from` to `to` is unobstructed by any wall.
pub fn has_los(from: (f32, f32), to: (f32, f32), walls: &[Wall]) -> bool {
    let dx = to.0 - from.0;
    let dy = to.1 - from.1;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 0.001 {
        return true;
    }
    let dir = (dx / len, dy / len);
    walls
        .iter()
        .all(|w| ray_aabb(from, dir, w).map_or(true, |t| t <= px_to_tiles(0.5) || t >= len))
}

/// Walk a ray and return the distance to the first wall hit (or `max_dist`).
pub fn cast_ray(origin: (f32, f32), dir: (f32, f32), max_dist: f32, walls: &[Wall]) -> f32 {
    walls
        .iter()
        .filter_map(|w| ray_aabb(origin, dir, w))
        .filter(|&t| t > px_to_tiles(0.5))
        .fold(max_dist, f32::min)
}

/// Ray–AABB intersection using the slab method.
/// Returns the entry distance along `dir`, or `None` if no hit.
pub fn ray_aabb(origin: (f32, f32), dir: (f32, f32), wall: &Wall) -> Option<f32> {
    let inv_x = if dir.0.abs() > 1e-9 {
        1.0 / dir.0
    } else {
        f32::INFINITY
    };
    let inv_y = if dir.1.abs() > 1e-9 {
        1.0 / dir.1
    } else {
        f32::INFINITY
    };

    let tx1 = (wall.x - origin.0) * inv_x;
    let tx2 = (wall.x + wall.w - origin.0) * inv_x;
    let ty1 = (wall.y - origin.1) * inv_y;
    let ty2 = (wall.y + wall.h - origin.1) * inv_y;

    let tmin = tx1.min(tx2).max(ty1.min(ty2));
    let tmax = tx1.max(tx2).min(ty1.max(ty2));

    if tmax >= tmin && tmin > 0.0 {
        Some(tmin)
    } else {
        None
    }
}
