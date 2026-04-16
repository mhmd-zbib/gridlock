use std::collections::HashSet;
use std::collections::VecDeque;

use super::grid::{blocked_depth, is_blocked, span};
use super::types::{DoorCluster, OUTSIDE_REGION_ID, RoomGap};
use super::{idx, neighbors4, neighbors8};

/// Doorway width constraints in cells (narrow openings that separate rooms).
const DOOR_MIN_WIDTH_CELLS: i32 = 1;
const DOOR_MAX_WIDTH_CELLS: i32 = 5;
/// Door candidate must connect into wider space on at least one axis.
const DOOR_EXPANSION_MIN_CELLS: i32 = 6;
/// Door candidate cluster shape limits.
const DOOR_CLUSTER_MAX_MAJOR_CELLS: i32 = 12;
const DOOR_CLUSTER_MAX_AREA_CELLS: usize = 80;
/// Separator walls around a doorway must continue this far.
const DOOR_WALL_CONTINUATION_MIN_CELLS: i32 = 5;
/// Merge nearby gap waypoints.
const GAP_DEDUP_PX: f32 = crate::world::units::px_to_tiles(50.0);

pub fn detect_door_clusters(free: &[bool], grid_w: i32, grid_h: i32) -> Vec<DoorCluster> {
    let mut candidate = vec![false; free.len()];
    let scan_limit = (DOOR_EXPANSION_MIN_CELLS + DOOR_MAX_WIDTH_CELLS + 2).max(8);

    for gy in 0..grid_h {
        for gx in 0..grid_w {
            if !free[idx(grid_w, gx, gy)] {
                continue;
            }

            let left = span(free, grid_w, grid_h, gx, gy, -1, 0, scan_limit);
            let right = span(free, grid_w, grid_h, gx, gy, 1, 0, scan_limit);
            let up = span(free, grid_w, grid_h, gx, gy, 0, -1, scan_limit);
            let down = span(free, grid_w, grid_h, gx, gy, 0, 1, scan_limit);

            let width_h = 1 + left + right;
            let width_v = 1 + up + down;
            let local_width = width_h.min(width_v);
            let long_axis = width_h.max(width_v);

            let is_narrow =
                local_width >= DOOR_MIN_WIDTH_CELLS && local_width <= DOOR_MAX_WIDTH_CELLS;
            let reaches_open_space = long_axis >= DOOR_EXPANSION_MIN_CELLS;

            if is_narrow && reaches_open_space {
                candidate[idx(grid_w, gx, gy)] = true;
            }
        }
    }

    let mut visited = vec![false; free.len()];
    let mut clusters = Vec::new();

    for gy in 0..grid_h {
        for gx in 0..grid_w {
            let start = idx(grid_w, gx, gy);
            if visited[start] || !candidate[start] {
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

            while let Some((cx, cy)) = q.pop_front() {
                cells.push((cx, cy));
                min_x = min_x.min(cx);
                min_y = min_y.min(cy);
                max_x = max_x.max(cx);
                max_y = max_y.max(cy);

                for (nx, ny) in neighbors8(cx, cy) {
                    if nx < 0 || ny < 0 || nx >= grid_w || ny >= grid_h {
                        continue;
                    }
                    let ni = idx(grid_w, nx, ny);
                    if visited[ni] || !candidate[ni] {
                        continue;
                    }
                    visited[ni] = true;
                    q.push_back((nx, ny));
                }
            }

            let bbox_w = max_x - min_x + 1;
            let bbox_h = max_y - min_y + 1;
            let minor = bbox_w.min(bbox_h);
            let major = bbox_w.max(bbox_h);

            if minor <= DOOR_MAX_WIDTH_CELLS
                && major <= DOOR_CLUSTER_MAX_MAJOR_CELLS
                && cells.len() <= DOOR_CLUSTER_MAX_AREA_CELLS
            {
                clusters.push(DoorCluster {
                    cells,
                    min_x,
                    min_y,
                    max_x,
                    max_y,
                });
            }
        }
    }

    clusters
}

pub fn should_seal_cluster(
    cluster: &DoorCluster,
    free: &[bool],
    outside_unsealed: &[bool],
    grid_w: i32,
    grid_h: i32,
) -> bool {
    cluster_touches_outside(cluster, free, outside_unsealed, grid_w, grid_h)
        || cluster_has_wall_continuation(cluster, free, grid_w, grid_h)
}

fn cluster_touches_outside(
    cluster: &DoorCluster,
    free: &[bool],
    outside_unsealed: &[bool],
    grid_w: i32,
    grid_h: i32,
) -> bool {
    let in_cluster: HashSet<(i32, i32)> = cluster.cells.iter().copied().collect();

    for &(cx, cy) in &cluster.cells {
        for (nx, ny) in neighbors8(cx, cy) {
            if nx < 0 || ny < 0 || nx >= grid_w || ny >= grid_h || in_cluster.contains(&(nx, ny)) {
                continue;
            }
            let ni = idx(grid_w, nx, ny);
            if !free[ni] {
                continue;
            }
            if outside_unsealed[ni] {
                return true;
            }
        }
    }

    false
}

fn cluster_has_wall_continuation(
    cluster: &DoorCluster,
    free: &[bool],
    grid_w: i32,
    grid_h: i32,
) -> bool {
    let bbox_w = cluster.max_x - cluster.min_x + 1;
    let bbox_h = cluster.max_y - cluster.min_y + 1;

    if bbox_w >= bbox_h {
        let y_up = cluster.min_y - 1;
        let y_down = cluster.max_y + 1;
        if y_up < 0 || y_down >= grid_h {
            return false;
        }

        let mut up_depth = 0;
        let mut down_depth = 0;
        for x in cluster.min_x..=cluster.max_x {
            if is_blocked(free, grid_w, x, y_up) {
                up_depth = up_depth.max(blocked_depth(free, grid_w, grid_h, x, y_up, 0, -1));
            }
            if is_blocked(free, grid_w, x, y_down) {
                down_depth = down_depth.max(blocked_depth(free, grid_w, grid_h, x, y_down, 0, 1));
            }
        }

        up_depth >= DOOR_WALL_CONTINUATION_MIN_CELLS
            && down_depth >= DOOR_WALL_CONTINUATION_MIN_CELLS
    } else {
        let x_left = cluster.min_x - 1;
        let x_right = cluster.max_x + 1;
        if x_left < 0 || x_right >= grid_w {
            return false;
        }

        let mut left_depth = 0;
        let mut right_depth = 0;
        for y in cluster.min_y..=cluster.max_y {
            if is_blocked(free, grid_w, x_left, y) {
                left_depth = left_depth.max(blocked_depth(free, grid_w, grid_h, x_left, y, -1, 0));
            }
            if is_blocked(free, grid_w, x_right, y) {
                right_depth =
                    right_depth.max(blocked_depth(free, grid_w, grid_h, x_right, y, 1, 0));
            }
        }

        left_depth >= DOOR_WALL_CONTINUATION_MIN_CELLS
            && right_depth >= DOOR_WALL_CONTINUATION_MIN_CELLS
    }
}

pub fn extract_gap_edges(
    door_clusters: &[DoorCluster],
    free: &[bool],
    outside: &[bool],
    room_lookup: &[i32],
    grid_w: i32,
    grid_h: i32,
) -> Vec<RoomGap> {
    let cell_size = super::CELL_SIZE;
    let mut edges = Vec::new();

    for cluster in door_clusters {
        let mut endpoints: HashSet<i32> = HashSet::new();
        for &(cx, cy) in &cluster.cells {
            for (nx, ny) in neighbors4(cx, cy) {
                if nx < 0 || ny < 0 || nx >= grid_w || ny >= grid_h {
                    continue;
                }
                let ni = idx(grid_w, nx, ny);
                if !free[ni] {
                    continue;
                }
                let rid = room_lookup[ni];
                if rid >= 0 {
                    endpoints.insert(rid);
                } else if outside[ni] {
                    endpoints.insert(OUTSIDE_REGION_ID);
                }
            }
        }

        if endpoints.len() < 2 {
            continue;
        }

        let bbox_w = cluster.max_x - cluster.min_x + 1;
        let bbox_h = cluster.max_y - cluster.min_y + 1;
        let width_cells = bbox_w.min(bbox_h).max(1) as usize;
        if width_cells < DOOR_MIN_WIDTH_CELLS as usize
            || width_cells > DOOR_MAX_WIDTH_CELLS as usize
        {
            continue;
        }

        let (cx, cy) = cluster_centroid(cluster);
        let pos = ((cx + 0.5) * cell_size, (cy + 0.5) * cell_size);

        let mut ep: Vec<i32> = endpoints.into_iter().collect();
        ep.sort_unstable();

        for i in 0..ep.len() {
            for j in (i + 1)..ep.len() {
                let (from_room, to_room) = normalize_pair(ep[i], ep[j]);
                upsert_gap_edge(
                    &mut edges,
                    RoomGap {
                        from_room,
                        to_room,
                        pos,
                        width_cells,
                    },
                );
            }
        }
    }

    edges
}

pub fn dedup_gap_points(points: impl Iterator<Item = (f32, f32)>) -> Vec<(f32, f32)> {
    let mut out: Vec<(f32, f32)> = Vec::new();
    'points: for p in points {
        for e in &out {
            let dx = p.0 - e.0;
            let dy = p.1 - e.1;
            if (dx * dx + dy * dy).sqrt() < GAP_DEDUP_PX {
                continue 'points;
            }
        }
        out.push(p);
    }
    out
}

fn cluster_centroid(cluster: &DoorCluster) -> (f32, f32) {
    let mut sx = 0.0_f32;
    let mut sy = 0.0_f32;
    for &(x, y) in &cluster.cells {
        sx += x as f32;
        sy += y as f32;
    }
    let n = cluster.cells.len().max(1) as f32;
    (sx / n, sy / n)
}

fn normalize_pair(a: i32, b: i32) -> (Option<usize>, Option<usize>) {
    let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
    (region_id_to_room(lo), region_id_to_room(hi))
}

fn region_id_to_room(id: i32) -> Option<usize> {
    if id == OUTSIDE_REGION_ID {
        None
    } else {
        Some(id as usize)
    }
}

fn upsert_gap_edge(edges: &mut Vec<RoomGap>, new_edge: RoomGap) {
    for existing in edges.iter_mut() {
        if existing.from_room == new_edge.from_room && existing.to_room == new_edge.to_room {
            let dx = existing.pos.0 - new_edge.pos.0;
            let dy = existing.pos.1 - new_edge.pos.1;
            if (dx * dx + dy * dy).sqrt() < GAP_DEDUP_PX {
                existing.pos = (
                    (existing.pos.0 + new_edge.pos.0) * 0.5,
                    (existing.pos.1 + new_edge.pos.1) * 0.5,
                );
                existing.width_cells = existing.width_cells.max(new_edge.width_cells);
                return;
            }
        }
    }
    edges.push(new_edge);
}
