use engine::render::quad::{QuadInstance, push_quad};
use game::world::level::LevelBounds;
use game::world::units::{px_to_tiles, tiles_to_px};

use crate::ui::editor::tool::{
    EDGE_STEP, GRID_LINE_ALPHA, GRID_LINE_THICKNESS, SUBGRID_DIVISIONS, SUBGRID_LINE_ALPHA,
    TILE_GRID,
};

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

    push_quad(out, (center_x, center_y), (w * 0.5, h * 0.5), fill_color);
    push_quad(
        out,
        (center_x, y + border * 0.5),
        (w * 0.5, border * 0.5),
        border_color,
    );
    push_quad(
        out,
        (center_x, y + h - border * 0.5),
        (w * 0.5, border * 0.5),
        border_color,
    );
    push_quad(
        out,
        (x + border * 0.5, center_y),
        (border * 0.5, h * 0.5),
        border_color,
    );
    push_quad(
        out,
        (x + w - border * 0.5, center_y),
        (border * 0.5, h * 0.5),
        border_color,
    );
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
        push_quad(out, (sx, half_h), (half_thickness, half_h), major_color);
    }

    let start_row = (view_origin.1 / TILE_GRID).floor() as i32;
    let end_row = ((view_origin.1 + view_h_tiles) / TILE_GRID).ceil() as i32;
    for row in start_row..=end_row {
        let world_y = row as f32 * TILE_GRID;
        let sy = (world_y - view_origin.1) * px_per_tile;
        push_quad(out, (half_w, sy), (half_w, half_thickness), major_color);
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
                push_quad(out, (sx, half_h), (sub_half_thickness, half_h), sub_color);
            }

            let start_sub_row = (view_origin.1 / sub_step_world).floor() as i32;
            let end_sub_row = ((view_origin.1 + view_h_tiles) / sub_step_world).ceil() as i32;
            for idx in start_sub_row..=end_sub_row {
                if idx.rem_euclid(SUBGRID_DIVISIONS as i32) == 0 {
                    continue;
                }
                let world_y = idx as f32 * sub_step_world;
                let sy = (world_y - view_origin.1) * px_per_tile;
                push_quad(out, (half_w, sy), (half_w, sub_half_thickness), sub_color);
            }
        }
    }
}
