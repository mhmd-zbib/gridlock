use crate::ui::editor::tool::{SUBGRID_DIVISIONS, SnapMode, TILE_GRID, Tool};

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
