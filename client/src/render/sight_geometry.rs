use game::world::ray::{cast_ray, wrap_angle};
use game::world::units::px_to_tiles;
use game::world::wall::Wall;

/// Cast rays around a circle and return wall-clipped outline points.
pub fn circle_arc_pts_raw(
    origin: (f32, f32),
    walls: &[Wall],
    n_rays: usize,
    radius: f32,
) -> Vec<[f32; 2]> {
    use std::f32::consts::{PI, TAU};

    let n = n_rays.max(8);
    let mut angles: Vec<f32> = (0..n).map(|i| -PI + (i as f32 / n as f32) * TAU).collect();

    const EPS: f32 = 0.0002;
    let range_sq = radius * radius;
    let skip_sq = px_to_tiles(1.0) * px_to_tiles(1.0);
    for w in walls {
        let closest_x = origin.0.clamp(w.x, w.x + w.w);
        let closest_y = origin.1.clamp(w.y, w.y + w.h);
        let cdx = closest_x - origin.0;
        let cdy = closest_y - origin.1;
        if cdx * cdx + cdy * cdy > range_sq {
            continue;
        }
        for &(cx, cy) in &[
            (w.x, w.y),
            (w.x + w.w, w.y),
            (w.x, w.y + w.h),
            (w.x + w.w, w.y + w.h),
        ] {
            let dx = cx - origin.0;
            let dy = cy - origin.1;
            if dx * dx + dy * dy < skip_sq {
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
            let dist = cast_ray(origin, dir, radius, walls);
            [origin.0 + dir.0 * dist, origin.1 + dir.1 * dist]
        })
        .collect();

    if let Some(first) = pts.first().copied() {
        pts.push(first);
    }

    pts
}

/// Cast rays across a directional cone and return wall-clipped arc endpoints.
pub fn cone_arc_pts_raw(
    origin: (f32, f32),
    walls: &[Wall],
    n_rays: usize,
    direction: f32,
    half_angle: f32,
    range: f32,
) -> Vec<[f32; 2]> {
    let mut rel: Vec<f32> = (0..=n_rays)
        .map(|i| {
            let t = i as f32 / n_rays as f32;
            -half_angle + t * half_angle * 2.0
        })
        .collect();

    const EPS: f32 = 0.0002;
    let range_sq = range * range;
    let skip_sq = px_to_tiles(1.0) * px_to_tiles(1.0);
    for w in walls {
        let closest_x = origin.0.clamp(w.x, w.x + w.w);
        let closest_y = origin.1.clamp(w.y, w.y + w.h);
        let cdx = closest_x - origin.0;
        let cdy = closest_y - origin.1;
        if cdx * cdx + cdy * cdy > range_sq {
            continue;
        }
        for &(cx, cy) in &[
            (w.x, w.y),
            (w.x + w.w, w.y),
            (w.x, w.y + w.h),
            (w.x + w.w, w.y + w.h),
        ] {
            let dx = cx - origin.0;
            let dy = cy - origin.1;
            if dx * dx + dy * dy < skip_sq {
                continue;
            }
            let r = wrap_angle(dy.atan2(dx) - direction);
            if r.abs() < half_angle {
                rel.push(r - EPS);
                rel.push(r);
                rel.push(r + EPS);
            }
        }
    }

    rel.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    rel.iter()
        .map(|&r| {
            let angle = direction + r.clamp(-half_angle, half_angle);
            let dir = (angle.cos(), angle.sin());
            let dist = cast_ray(origin, dir, range, walls);
            [origin.0 + dir.0 * dist, origin.1 + dir.1 * dist]
        })
        .collect()
}

/// Alias for `cone_arc_pts_raw` used for aim cone rendering.
///
/// Takes explicit `render_range` (weapon-dependent visual distance)
/// instead of a game sight range.
pub fn aim_cone_arc_pts(
    origin: (f32, f32),
    walls: &[Wall],
    n_rays: usize,
    direction: f32,
    half_angle: f32,
    render_range: f32,
) -> Vec<[f32; 2]> {
    cone_arc_pts_raw(origin, walls, n_rays, direction, half_angle, render_range)
}
