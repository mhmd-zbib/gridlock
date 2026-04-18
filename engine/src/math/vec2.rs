pub fn length(v: (f32, f32)) -> f32 {
    (v.0 * v.0 + v.1 * v.1).sqrt()
}

pub fn normalize(v: (f32, f32)) -> (f32, f32) {
    let len = length(v);
    if len <= f32::EPSILON {
        (0.0, 0.0)
    } else {
        (v.0 / len, v.1 / len)
    }
}

pub fn lerp(a: (f32, f32), b: (f32, f32), t: f32) -> (f32, f32) {
    let t = t.clamp(0.0, 1.0);
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

pub fn clamp_to_radius(point: (f32, f32), center: (f32, f32), radius: f32) -> (f32, f32) {
    let dx = point.0 - center.0;
    let dy = point.1 - center.1;
    let dist = (dx * dx + dy * dy).sqrt();
    if dist <= radius || dist <= f32::EPSILON {
        return point;
    }
    let scale = radius / dist;
    (center.0 + dx * scale, center.1 + dy * scale)
}
