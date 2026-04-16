use engine::render::quad::QuadInstance;
use game::world::level::LevelBounds;
use game::world::units::{px_to_tiles, tiles_to_px};
use game::world::wall::Wall;

use super::tool::{
    BREAKABLE_HP, BREAKABLE_THICKNESS, EDGE_STEP, EdgeAxis, EdgeCell, EdgeKey, GRID_LINE_ALPHA,
    GRID_LINE_THICKNESS, SUBGRID_DIVISIONS, SUBGRID_LINE_ALPHA, SnapMode, TILE_GRID, Tool,
    WALL_THICKNESS,
};

// ---------------------------------------------------------------------------
// Snap helpers
// ---------------------------------------------------------------------------

pub fn snap(v: f32, grid: f32) -> f32 {
    (v / grid).round() * grid
}

pub fn snap_point(x: f32, y: f32, mode: SnapMode) -> (f32, f32) {
    match mode {
        SnapMode::Edge => (snap(x, TILE_GRID), snap(y, TILE_GRID)),
        SnapMode::Center => (
            snap(x - TILE_GRID * 0.5, TILE_GRID) + TILE_GRID * 0.5,
            snap(y - TILE_GRID * 0.5, TILE_GRID) + TILE_GRID * 0.5,
        ),
        SnapMode::Subgrid => {
            let sub = TILE_GRID / SUBGRID_DIVISIONS as f32;
            (snap(x, sub), snap(y, sub))
        }
    }
}

pub fn effective_snap_mode(tool: Tool, mode: SnapMode) -> SnapMode {
    if matches!(tool, Tool::Wall | Tool::Breakable) && mode == SnapMode::Center {
        SnapMode::Edge
    } else {
        mode
    }
}

// ---------------------------------------------------------------------------
// Edge path helpers
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Geometry helpers
// ---------------------------------------------------------------------------

pub fn bounds_from_points(a: (f32, f32), b: (f32, f32)) -> Option<LevelBounds> {
    let min_x = a.0.min(b.0);
    let min_y = a.1.min(b.1);
    let max_x = a.0.max(b.0);
    let max_y = a.1.max(b.1);
    let w = max_x - min_x;
    let h = max_y - min_y;
    if w < EDGE_STEP || h < EDGE_STEP {
        return None;
    }
    Some(LevelBounds {
        x: min_x,
        y: min_y,
        w,
        h,
    })
}

pub fn push_bounds_fill(
    out: &mut Vec<QuadInstance>,
    bounds: LevelBounds,
    fill_color: [f32; 4],
    border_color: [f32; 4],
    view_origin: (f32, f32),
    zoom: f32,
) {
    let scale = tiles_to_px(1.0) * zoom;
    let x = (bounds.x - view_origin.0) * scale;
    let y = (bounds.y - view_origin.1) * scale;
    let w = bounds.w * scale;
    let h = bounds.h * scale;
    let center_x = x + w * 0.5;
    let center_y = y + h * 0.5;
    let border = 1.5;

    out.push(QuadInstance {
        center: [center_x, center_y],
        half_size: [w * 0.5, h * 0.5],
        color: fill_color,
    });

    out.push(QuadInstance {
        center: [center_x, y + border * 0.5],
        half_size: [w * 0.5, border * 0.5],
        color: border_color,
    });
    out.push(QuadInstance {
        center: [center_x, y + h - border * 0.5],
        half_size: [w * 0.5, border * 0.5],
        color: border_color,
    });
    out.push(QuadInstance {
        center: [x + border * 0.5, center_y],
        half_size: [border * 0.5, h * 0.5],
        color: border_color,
    });
    out.push(QuadInstance {
        center: [x + w - border * 0.5, center_y],
        half_size: [border * 0.5, h * 0.5],
        color: border_color,
    });
}

pub fn append_grid_lines(
    out: &mut Vec<QuadInstance>,
    screen_w: f32,
    screen_h: f32,
    show_subgrid: bool,
    view_origin: (f32, f32),
    zoom: f32,
) {
    let major_color = [0.85, 0.85, 0.95, GRID_LINE_ALPHA];
    let sub_color = [0.90, 0.90, 1.00, SUBGRID_LINE_ALPHA];
    let half_w = screen_w * 0.5;
    let half_h = screen_h * 0.5;
    let half_thickness = (tiles_to_px(GRID_LINE_THICKNESS) * zoom).clamp(1.0, 2.5) * 0.5;
    let sub_half_thickness = half_thickness;
    let px_per_tile = tiles_to_px(1.0) * zoom;
    let view_w_tiles = px_to_tiles(screen_w) / zoom.max(0.001);
    let view_h_tiles = px_to_tiles(screen_h) / zoom.max(0.001);

    let start_col = (view_origin.0 / TILE_GRID).floor() as i32;
    let end_col = ((view_origin.0 + view_w_tiles) / TILE_GRID).ceil() as i32;
    for col in start_col..=end_col {
        let world_x = col as f32 * TILE_GRID;
        let sx = (world_x - view_origin.0) * px_per_tile;
        out.push(QuadInstance {
            center: [sx, half_h],
            half_size: [half_thickness, half_h],
            color: major_color,
        });
    }

    let start_row = (view_origin.1 / TILE_GRID).floor() as i32;
    let end_row = ((view_origin.1 + view_h_tiles) / TILE_GRID).ceil() as i32;
    for row in start_row..=end_row {
        let world_y = row as f32 * TILE_GRID;
        let sy = (world_y - view_origin.1) * px_per_tile;
        out.push(QuadInstance {
            center: [half_w, sy],
            half_size: [half_w, half_thickness],
            color: major_color,
        });
    }

    if show_subgrid {
        let sub_step_world = TILE_GRID / SUBGRID_DIVISIONS as f32;
        let sub_step_px = sub_step_world * px_per_tile;
        if sub_step_px >= 4.0 {
            let start_sub_col = (view_origin.0 / sub_step_world).floor() as i32;
            let end_sub_col = ((view_origin.0 + view_w_tiles) / sub_step_world).ceil() as i32;
            for idx in start_sub_col..=end_sub_col {
                if idx.rem_euclid(SUBGRID_DIVISIONS as i32) == 0 {
                    continue;
                }
                let world_x = idx as f32 * sub_step_world;
                let sx = (world_x - view_origin.0) * px_per_tile;
                out.push(QuadInstance {
                    center: [sx, half_h],
                    half_size: [sub_half_thickness, half_h],
                    color: sub_color,
                });
            }

            let start_sub_row = (view_origin.1 / sub_step_world).floor() as i32;
            let end_sub_row = ((view_origin.1 + view_h_tiles) / sub_step_world).ceil() as i32;
            for idx in start_sub_row..=end_sub_row {
                if idx.rem_euclid(SUBGRID_DIVISIONS as i32) == 0 {
                    continue;
                }
                let world_y = idx as f32 * sub_step_world;
                let sy = (world_y - view_origin.1) * px_per_tile;
                out.push(QuadInstance {
                    center: [half_w, sy],
                    half_size: [half_w, sub_half_thickness],
                    color: sub_color,
                });
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Misc
// ---------------------------------------------------------------------------

pub fn dist(ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let dx = ax - bx;
    let dy = ay - by;
    (dx * dx + dy * dy).sqrt()
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

pub fn nearest_idx(
    points: &[game::world::level::Pos],
    x: f32,
    y: f32,
    max_dist: f32,
) -> Option<usize> {
    points
        .iter()
        .enumerate()
        .map(|(i, p)| (i, dist(p.x, p.y, x, y)))
        .filter(|(_, d)| *d < max_dist)
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .map(|(i, _)| i)
}
