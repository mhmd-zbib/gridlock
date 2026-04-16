use std::collections::VecDeque;

use super::types::{DetectedRoom, ROOM_COLORS};
use super::{idx, neighbors4};

pub fn flood_outside(sealed_free: &[bool], grid_w: i32, grid_h: i32) -> Vec<bool> {
    let mut outside = vec![false; sealed_free.len()];
    let mut q = VecDeque::new();

    let enqueue = |x: i32, y: i32, outside: &mut [bool], q: &mut VecDeque<(i32, i32)>| {
        let i = idx(grid_w, x, y);
        if sealed_free[i] && !outside[i] {
            outside[i] = true;
            q.push_back((x, y));
        }
    };

    for x in 0..grid_w {
        enqueue(x, 0, &mut outside, &mut q);
        enqueue(x, grid_h - 1, &mut outside, &mut q);
    }
    for y in 0..grid_h {
        enqueue(0, y, &mut outside, &mut q);
        enqueue(grid_w - 1, y, &mut outside, &mut q);
    }

    while let Some((cx, cy)) = q.pop_front() {
        for (nx, ny) in neighbors4(cx, cy) {
            if nx < 0 || ny < 0 || nx >= grid_w || ny >= grid_h {
                continue;
            }
            let ni = idx(grid_w, nx, ny);
            if sealed_free[ni] && !outside[ni] {
                outside[ni] = true;
                q.push_back((nx, ny));
            }
        }
    }

    outside
}

/// Minimum enclosed cell count to count as a room.
const MIN_ROOM_AREA_CELLS: usize = 10;

pub fn extract_rooms(
    sealed_free: &[bool],
    outside: &[bool],
    grid_w: i32,
    grid_h: i32,
    room_lookup: &mut [i32],
) -> Vec<DetectedRoom> {
    let cell_size = super::CELL_SIZE;
    let mut visited = vec![false; sealed_free.len()];
    let mut rooms = Vec::new();

    for gy in 0..grid_h {
        for gx in 0..grid_w {
            let start = idx(grid_w, gx, gy);
            if visited[start] || !sealed_free[start] || outside[start] {
                continue;
            }

            let mut q = VecDeque::new();
            q.push_back((gx, gy));
            visited[start] = true;

            let mut cells = Vec::new();
            let mut min_x = gx;
            let mut min_y = gy;
            let mut max_x = gx;
            let mut max_y = gy;
            let mut sum_x = 0.0_f32;
            let mut sum_y = 0.0_f32;

            while let Some((cx, cy)) = q.pop_front() {
                cells.push((cx, cy));
                min_x = min_x.min(cx);
                min_y = min_y.min(cy);
                max_x = max_x.max(cx);
                max_y = max_y.max(cy);
                sum_x += cx as f32 + 0.5;
                sum_y += cy as f32 + 0.5;

                for (nx, ny) in neighbors4(cx, cy) {
                    if nx < 0 || ny < 0 || nx >= grid_w || ny >= grid_h {
                        continue;
                    }
                    let ni = idx(grid_w, nx, ny);
                    if !visited[ni] && sealed_free[ni] && !outside[ni] {
                        visited[ni] = true;
                        q.push_back((nx, ny));
                    }
                }
            }

            if cells.len() < MIN_ROOM_AREA_CELLS {
                continue;
            }

            let room_id = rooms.len();
            for &(x, y) in &cells {
                room_lookup[idx(grid_w, x, y)] = room_id as i32;
            }

            let area_cells = cells.len();
            let centroid = (
                (sum_x / area_cells as f32) * cell_size,
                (sum_y / area_cells as f32) * cell_size,
            );

            rooms.push(DetectedRoom {
                id: room_id,
                x: min_x as f32 * cell_size,
                y: min_y as f32 * cell_size,
                w: (max_x - min_x + 1) as f32 * cell_size,
                h: (max_y - min_y + 1) as f32 * cell_size,
                color: ROOM_COLORS[room_id % ROOM_COLORS.len()],
                centroid,
                area_cells,
            });
        }
    }

    rooms
}
