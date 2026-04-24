use engine::math::vec2;
use game::world::level::Pos;

pub fn dist(ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    vec2::length((ax - bx, ay - by))
}

pub fn point_to_segment_dist(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let vx = bx - ax;
    let vy = by - ay;
    let wx = px - ax;
    let wy = py - ay;
    let c1 = vx * wx + vy * wy;
    if c1 <= 0.0 {
        return dist(px, py, ax, ay);
    }
    let c2 = vx * vx + vy * vy;
    if c2 <= c1 {
        return dist(px, py, bx, by);
    }
    let t = c1 / c2;
    dist(px, py, ax + t * vx, ay + t * vy)
}

pub fn nearest_idx(points: &[Pos], x: f32, y: f32, max_dist: f32) -> Option<usize> {
    points
        .iter()
        .enumerate()
        .map(|(i, p)| (i, dist(p.x, p.y, x, y)))
        .filter(|(_, d)| *d < max_dist)
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .map(|(i, _)| i)
}
