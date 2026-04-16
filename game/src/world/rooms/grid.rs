use crate::world::units::px_to_tiles;
use crate::world::wall::Wall;

/// Inflate blocked space by one cell so detected rooms are navigable for actors.
pub const COLLISION_CLEARANCE_CELLS: i32 = 1;

pub fn build_blocked_grid(walls: &[Wall], grid_w: i32, grid_h: i32) -> Vec<bool> {
    let cell_size = super::CELL_SIZE;
    let mut blocked = vec![false; (grid_w * grid_h) as usize];
    for gy in 0..grid_h {
        for gx in 0..grid_w {
            let wx = (gx as f32 + 0.5) * cell_size;
            let wy = (gy as f32 + 0.5) * cell_size;
            if walls.iter().any(|w| w.overlaps(wx, wy, px_to_tiles(1.0))) {
                blocked[super::idx(grid_w, gx, gy)] = true;
            }
        }
    }

    if COLLISION_CLEARANCE_CELLS <= 0 {
        return blocked;
    }

    let mut inflated = blocked.clone();
    for gy in 0..grid_h {
        for gx in 0..grid_w {
            if !blocked[super::idx(grid_w, gx, gy)] {
                continue;
            }
            for oy in -COLLISION_CLEARANCE_CELLS..=COLLISION_CLEARANCE_CELLS {
                for ox in -COLLISION_CLEARANCE_CELLS..=COLLISION_CLEARANCE_CELLS {
                    let nx = gx + ox;
                    let ny = gy + oy;
                    if nx >= 0 && ny >= 0 && nx < grid_w && ny < grid_h {
                        inflated[super::idx(grid_w, nx, ny)] = true;
                    }
                }
            }
        }
    }

    inflated
}

pub fn is_blocked(free: &[bool], grid_w: i32, x: i32, y: i32) -> bool {
    !free[super::idx(grid_w, x, y)]
}

pub fn blocked_depth(
    free: &[bool],
    grid_w: i32,
    grid_h: i32,
    x: i32,
    y: i32,
    dx: i32,
    dy: i32,
) -> i32 {
    let mut depth = 0;
    let mut cx = x;
    let mut cy = y;
    while cx >= 0 && cy >= 0 && cx < grid_w && cy < grid_h && is_blocked(free, grid_w, cx, cy) {
        depth += 1;
        cx += dx;
        cy += dy;
    }
    depth
}

pub fn span(
    free: &[bool],
    grid_w: i32,
    grid_h: i32,
    x: i32,
    y: i32,
    dx: i32,
    dy: i32,
    max_steps: i32,
) -> i32 {
    let mut steps = 0;
    let mut cx = x;
    let mut cy = y;
    while steps < max_steps {
        cx += dx;
        cy += dy;
        if cx < 0 || cy < 0 || cx >= grid_w || cy >= grid_h {
            break;
        }
        if !free[super::idx(grid_w, cx, cy)] {
            break;
        }
        steps += 1;
    }
    steps
}
