use game::world::wall::Wall;

use super::misc::dist;
use super::snap::snap_point;
use crate::ui::editor::tool::{
    BREAKABLE_HP, BREAKABLE_THICKNESS, EDGE_STEP, EdgeAxis, EdgeCell, EdgeKey, SnapMode,
    WALL_THICKNESS,
};

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

pub fn edge_to_wall_rect(edge: EdgeKey, breakable: bool) -> (f32, f32, f32, f32) {
    let thickness = if breakable {
        BREAKABLE_THICKNESS
    } else {
        WALL_THICKNESS
    };
    match edge.axis {
        EdgeAxis::Horizontal => (
            edge.x as f32 * EDGE_STEP,
            edge.y as f32 * EDGE_STEP - thickness * 0.5,
            EDGE_STEP,
            thickness,
        ),
        EdgeAxis::Vertical => (
            edge.x as f32 * EDGE_STEP - thickness * 0.5,
            edge.y as f32 * EDGE_STEP,
            thickness,
            EDGE_STEP,
        ),
    }
}

pub fn choose_corner_cell(cells: &[EdgeCell]) -> EdgeCell {
    let all_breakable = cells.iter().all(|c| c.breakable);
    if all_breakable {
        let hp = cells.iter().map(|c| c.hp).min().unwrap_or(1).max(1);
        EdgeCell {
            breakable: true,
            hp,
        }
    } else {
        EdgeCell {
            breakable: false,
            hp: 1,
        }
    }
}

pub fn build_corner_patch(vx: i32, vy: i32, cell: EdgeCell) -> Wall {
    let t = if cell.breakable {
        BREAKABLE_THICKNESS
    } else {
        WALL_THICKNESS
    };
    let x = vx as f32 * EDGE_STEP - t * 0.5;
    let y = vy as f32 * EDGE_STEP - t * 0.5;
    if cell.breakable {
        Wall::new_breakable(x, y, t, t, cell.hp)
    } else {
        Wall::new(x, y, t, t)
    }
}

pub fn preview_edge_walls(
    start: (f32, f32),
    end: (f32, f32),
    mode: SnapMode,
    breakable: bool,
) -> Vec<Wall> {
    let Some((keys, _)) = collect_edge_path(start, end, mode) else {
        return Vec::new();
    };
    keys.into_iter()
        .map(|k| {
            let (x, y, w, h) = edge_to_wall_rect(k, breakable);
            if breakable {
                Wall::new_breakable(x, y, w, h, BREAKABLE_HP)
            } else {
                Wall::new(x, y, w, h)
            }
        })
        .collect()
}
