use game::world::wall::Wall;

use super::misc::dist;
use super::snap::snap_point;
use crate::ui::editor::tool::{BREAKABLE_HP, EDGE_STEP, EdgeAxis, EdgeCell, EdgeKey, SnapMode, WallShape};

#[derive(Clone, Copy)]
pub struct GridVertex {
    pub x: i32,
    pub y: i32,
}

pub fn point_to_edge_vertex(p: (f32, f32), mode: SnapMode) -> GridVertex {
    let (sx, sy) = snap_point(p.0, p.1, mode);
    GridVertex {
        x: (sx / EDGE_STEP).round() as i32,
        y: (sy / EDGE_STEP).round() as i32,
    }
}

pub fn edge_vertex_to_point(v: GridVertex) -> (f32, f32) {
    (v.x as f32 * EDGE_STEP, v.y as f32 * EDGE_STEP)
}

pub fn collect_edge_path(
    start: (f32, f32),
    end: (f32, f32),
    mode: SnapMode,
) -> Option<(Vec<EdgeKey>, f32)> {
    let mut a = point_to_edge_vertex(start, mode);
    let mut b = point_to_edge_vertex(end, mode);

    let dx = b.x - a.x;
    let dy = b.y - a.y;
    if dx.abs() >= dy.abs() {
        b.y = a.y;
    } else {
        b.x = a.x;
    }

    if a.x == b.x && a.y == b.y {
        return None;
    }

    if a.x > b.x || (a.x == b.x && a.y > b.y) {
        std::mem::swap(&mut a, &mut b);
    }

    let mut keys = Vec::new();
    if a.y == b.y {
        for x in a.x..b.x {
            keys.push(EdgeKey {
                axis: EdgeAxis::Horizontal,
                x,
                y: a.y,
            });
        }
    } else {
        for y in a.y..b.y {
            keys.push(EdgeKey {
                axis: EdgeAxis::Vertical,
                x: a.x,
                y,
            });
        }
    }

    let pa = edge_vertex_to_point(a);
    let pb = edge_vertex_to_point(b);
    Some((keys, dist(pa.0, pa.1, pb.0, pb.1)))
}

/// Find the nearest edge segment (axis + cell coords) for a raw cursor position.
/// Uses the raw world position (not snapped) so it picks whichever grid line is
/// physically closer to the cursor, giving an accurate visual preview.
pub fn cursor_edge_key(cx: f32, cy: f32) -> EdgeKey {
    let nearest_y = (cy / EDGE_STEP).round() * EDGE_STEP;
    let dist_h = (cy - nearest_y).abs();

    let nearest_x = (cx / EDGE_STEP).round() * EDGE_STEP;
    let dist_v = (cx - nearest_x).abs();

    if dist_h <= dist_v {
        EdgeKey {
            axis: EdgeAxis::Horizontal,
            x: (cx / EDGE_STEP).floor() as i32,
            y: (nearest_y / EDGE_STEP).round() as i32,
        }
    } else {
        EdgeKey {
            axis: EdgeAxis::Vertical,
            x: (nearest_x / EDGE_STEP).round() as i32,
            y: (cy / EDGE_STEP).floor() as i32,
        }
    }
}

/// Convert an edge segment to a wall rect (x, y, w, h).
/// The wall starts at the grid line and extends inward (downward for horizontal,
/// rightward for vertical) by `thickness` tiles.
pub fn edge_to_wall_rect(edge: EdgeKey, thickness: f32) -> (f32, f32, f32, f32) {
    match edge.axis {
        EdgeAxis::Horizontal => (
            edge.x as f32 * EDGE_STEP,
            edge.y as f32 * EDGE_STEP,
            EDGE_STEP,
            thickness,
        ),
        EdgeAxis::Vertical => (
            edge.x as f32 * EDGE_STEP,
            edge.y as f32 * EDGE_STEP,
            thickness,
            EDGE_STEP,
        ),
    }
}

pub fn choose_corner_cell(cells: &[EdgeCell]) -> EdgeCell {
    let all_breakable = cells.iter().all(|c| c.breakable);
    let max_thickness = cells.iter().map(|c| c.thickness_steps).max().unwrap_or(1);
    if all_breakable {
        let hp = cells.iter().map(|c| c.hp).min().unwrap_or(1).max(1);
        EdgeCell {
            breakable: true,
            hp,
            thickness_steps: max_thickness,
        }
    } else {
        EdgeCell {
            breakable: false,
            hp: 1,
            thickness_steps: max_thickness,
        }
    }
}

pub fn build_corner_patch(vx: i32, vy: i32, cell: EdgeCell) -> Wall {
    let t = cell.thickness_steps as f32 * EDGE_STEP;
    let x = vx as f32 * EDGE_STEP;
    let y = vy as f32 * EDGE_STEP;
    if cell.breakable {
        Wall::new_breakable(x, y, t, t, cell.hp)
    } else {
        Wall::new(x, y, t, t)
    }
}

/// Returns (x, y, w, h) wall rects for a single-click tile placement at (cx, cy).
pub fn preview_wall_tile(cx: f32, cy: f32, shape: WallShape) -> Vec<(f32, f32, f32, f32)> {
    let axis = if shape.x_steps > shape.y_steps {
        EdgeAxis::Horizontal
    } else if shape.y_steps > shape.x_steps {
        EdgeAxis::Vertical
    } else {
        cursor_edge_key(cx, cy).axis
    };

    let (thickness_steps, num_edges) = match axis {
        EdgeAxis::Horizontal => (shape.y_steps, shape.x_steps),
        EdgeAxis::Vertical => (shape.x_steps, shape.y_steps),
    };
    let thickness = thickness_steps as f32 * EDGE_STEP;

    let mut rects = Vec::new();
    match axis {
        EdgeAxis::Horizontal => {
            let y = (cy / EDGE_STEP).round() as i32;
            let cx_grid = (cx / EDGE_STEP).round() as i32;
            let x_start = cx_grid - (num_edges as i32) / 2;
            for i in 0..num_edges as i32 {
                let key = EdgeKey { axis: EdgeAxis::Horizontal, x: x_start + i, y };
                rects.push(edge_to_wall_rect(key, thickness));
            }
        }
        EdgeAxis::Vertical => {
            let x = (cx / EDGE_STEP).round() as i32;
            let cy_grid = (cy / EDGE_STEP).round() as i32;
            let y_start = cy_grid - (num_edges as i32) / 2;
            for i in 0..num_edges as i32 {
                let key = EdgeKey { axis: EdgeAxis::Vertical, x, y: y_start + i };
                rects.push(edge_to_wall_rect(key, thickness));
            }
        }
    }
    rects
}

pub fn preview_edge_walls(
    start: (f32, f32),
    end: (f32, f32),
    mode: SnapMode,
    breakable: bool,
    thickness_steps: u32,
) -> Vec<Wall> {
    let Some((keys, _)) = collect_edge_path(start, end, mode) else {
        return Vec::new();
    };
    let thickness = thickness_steps as f32 * EDGE_STEP;
    keys.into_iter()
        .map(|k| {
            let (x, y, w, h) = edge_to_wall_rect(k, thickness);
            if breakable {
                Wall::new_breakable(x, y, w, h, BREAKABLE_HP)
            } else {
                Wall::new(x, y, w, h)
            }
        })
        .collect()
}
