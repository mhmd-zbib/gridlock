use bytemuck::{Pod, Zeroable};

pub const MAX_VERTS: u64 = 65_536 * 3; // 65k triangles per frame

/// One coloured vertex for a triangle.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GeoVertex {
    pub pos: [f32; 2],
    pub color: [f32; 4],
}

/// Build a filled circle disk as a triangle fan.
pub fn push_circle_fan(
    out: &mut Vec<GeoVertex>,
    center: (f32, f32),
    radius: f32,
    color: [f32; 4],
    n: usize,
) {
    use std::f32::consts::TAU;
    let c = GeoVertex { pos: [center.0, center.1], color };
    for i in 1..=n {
        let a0 = (i - 1) as f32 / n as f32 * TAU;
        let a1 = i as f32 / n as f32 * TAU;
        out.push(c);
        out.push(GeoVertex { pos: [center.0 + a0.cos() * radius, center.1 + a0.sin() * radius], color });
        out.push(GeoVertex { pos: [center.0 + a1.cos() * radius, center.1 + a1.sin() * radius], color });
    }
}

pub fn push_cone_fan(
    out: &mut Vec<GeoVertex>,
    center: (f32, f32),
    arc: &[[f32; 2]],
    color: [f32; 4],
) {
    let c = GeoVertex {
        pos: [center.0, center.1],
        color,
    };
    for i in 1..arc.len() {
        out.push(c);
        out.push(GeoVertex {
            pos: arc[i - 1],
            color,
        });
        out.push(GeoVertex { pos: arc[i], color });
    }
}

/// Push a filled axis-aligned rectangle in screen space.
pub fn push_rect(
    out: &mut Vec<GeoVertex>,
    top_left: (f32, f32),
    bottom_right: (f32, f32),
    color: [f32; 4],
) {
    let (x, y) = top_left;
    let (x2, y2) = bottom_right;
    out.push(GeoVertex { pos: [x, y], color });
    out.push(GeoVertex {
        pos: [x2, y],
        color,
    });
    out.push(GeoVertex {
        pos: [x, y2],
        color,
    });
    out.push(GeoVertex {
        pos: [x2, y],
        color,
    });
    out.push(GeoVertex {
        pos: [x2, y2],
        color,
    });
    out.push(GeoVertex {
        pos: [x, y2],
        color,
    });
}

/// Push a filled diamond centered at `center`.
pub fn push_diamond(out: &mut Vec<GeoVertex>, center: (f32, f32), radius: f32, color: [f32; 4]) {
    let (cx, cy) = center;
    let n = [cx, cy - radius];
    let e = [cx + radius, cy];
    let s = [cx, cy + radius];
    let w = [cx - radius, cy];

    out.push(GeoVertex { pos: n, color });
    out.push(GeoVertex { pos: e, color });
    out.push(GeoVertex { pos: s, color });
    out.push(GeoVertex { pos: n, color });
    out.push(GeoVertex { pos: s, color });
    out.push(GeoVertex { pos: w, color });
}

/// Push a filled line segment as a quad (two triangles).
pub fn push_line_segment(
    out: &mut Vec<GeoVertex>,
    a: (f32, f32),
    b: (f32, f32),
    width: f32,
    color: [f32; 4],
) {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 0.5 {
        return;
    }
    let half = width * 0.5;
    let nx = -dy / len * half;
    let ny = dx / len * half;

    let v0 = [a.0 + nx, a.1 + ny];
    let v1 = [a.0 - nx, a.1 - ny];
    let v2 = [b.0 + nx, b.1 + ny];
    let v3 = [b.0 - nx, b.1 - ny];

    out.push(GeoVertex { pos: v0, color });
    out.push(GeoVertex { pos: v1, color });
    out.push(GeoVertex { pos: v2, color });
    out.push(GeoVertex { pos: v1, color });
    out.push(GeoVertex { pos: v3, color });
    out.push(GeoVertex { pos: v2, color });
}
