use crate::core::world::units::px_to_tiles;
use crate::core::world::wall::Wall;
use std::collections::{HashSet, VecDeque};

/// Grid cell size for occupancy / topology analysis (world tiles).
const CELL_SIZE: f32 = px_to_tiles(8.0);
/// Inflate blocked space by one cell so detected rooms are navigable for actors.
const COLLISION_CLEARANCE_CELLS: i32 = 1;
/// Ignore tiny enclosed pockets produced by discretization noise.
const MIN_ROOM_AREA_CELLS: usize = 10;
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
const GAP_DEDUP_PX: f32 = px_to_tiles(50.0);

const ROOM_COLORS: [[f32; 4]; 6] = [
    [0.28, 0.52, 0.90, 0.12],
    [0.28, 0.78, 0.40, 0.12],
    [0.88, 0.44, 0.28, 0.12],
    [0.64, 0.28, 0.84, 0.12],
    [0.84, 0.72, 0.18, 0.12],
    [0.18, 0.72, 0.72, 0.12],
];

const OUTSIDE_REGION_ID: i32 = -1;

pub struct DetectedRoom {
    pub id: usize,
    /// Room AABB in world space.
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub color: [f32; 4],
    pub centroid: (f32, f32),
    pub area_cells: usize,
}

pub struct RoomGap {
    /// `None` means outside region.
    pub from_room: Option<usize>,
    /// `None` means outside region.
    pub to_room: Option<usize>,
    pub pos: (f32, f32),
    pub width_cells: usize,
}

pub struct LevelRooms {
    pub rooms: Vec<DetectedRoom>,
    /// Deduplicated gap / opening waypoints (world space) for debug rendering.
    pub gaps: Vec<(f32, f32)>,
    /// Explicit topological door/gap edges between regions.
    pub gap_edges: Vec<RoomGap>,
    pub outside_cells: usize,

    grid_w: i32,
    grid_h: i32,
    cell_size: f32,
    room_lookup: Vec<i32>,
}

impl Default for LevelRooms {
    fn default() -> Self {
        Self {
            rooms: Vec::new(),
            gaps: Vec::new(),
            gap_edges: Vec::new(),
            outside_cells: 0,
            grid_w: 0,
            grid_h: 0,
            cell_size: CELL_SIZE,
            room_lookup: Vec::new(),
        }
    }
}

impl LevelRooms {
    /// Analyse `walls` over the area `(0,0)..(level_w, level_h)`.
    /// The pipeline is:
    /// 1) occupancy grid
    /// 2) narrow-door detection
    /// 3) temporary door sealing
    /// 4) outside flood-fill
    /// 5) enclosed component extraction (rooms)
    /// 6) doorway edge extraction + dedup
    pub fn compute(walls: &[Wall], level_w: f32, level_h: f32) -> Self {
        detect_rooms_and_gaps(walls, level_w, level_h)
    }

    /// Fast room lookup by world position using cached room-id grid.
    pub fn find_room_at(&self, x: f32, y: f32) -> Option<usize> {
        if self.grid_w > 0 && self.grid_h > 0 && !self.room_lookup.is_empty() {
            let gx = (x / self.cell_size).floor() as i32;
            let gy = (y / self.cell_size).floor() as i32;
            if gx >= 0 && gy >= 0 && gx < self.grid_w && gy < self.grid_h {
                let rid = self.room_lookup[idx(self.grid_w, gx, gy)];
                if rid >= 0 {
                    return Some(rid as usize);
                }
            }
        }

        // Fallback for edge precision around grid-cell boundaries.
        self.rooms.iter().find_map(|room| {
            if x >= room.x && x < room.x + room.w && y >= room.y && y < room.y + room.h {
                Some(room.id)
            } else {
                None
            }
        })
    }
}

#[derive(Clone)]
struct DoorCluster {
    cells: Vec<(i32, i32)>,
    min_x: i32,
    min_y: i32,
    max_x: i32,
    max_y: i32,
}

fn detect_rooms_and_gaps(walls: &[Wall], level_w: f32, level_h: f32) -> LevelRooms {
    let grid_w = ((level_w / CELL_SIZE).ceil() as i32).max(1);
    let grid_h = ((level_h / CELL_SIZE).ceil() as i32).max(1);
    let cell_count = (grid_w * grid_h) as usize;

    let blocked = build_blocked_grid(walls, grid_w, grid_h);
    let free: Vec<bool> = blocked.iter().map(|b| !b).collect();

    let outside_unsealed = flood_outside(&free, grid_w, grid_h);
    let door_clusters: Vec<DoorCluster> = detect_door_clusters(&free, grid_w, grid_h)
        .into_iter()
        .filter(|cluster| should_seal_cluster(cluster, &free, &outside_unsealed, grid_w, grid_h))
        .collect();

    let mut sealed_free = free.clone();
    for cluster in &door_clusters {
        for &(x, y) in &cluster.cells {
            sealed_free[idx(grid_w, x, y)] = false;
        }
    }

    let outside = flood_outside(&sealed_free, grid_w, grid_h);
    let outside_cells = outside.iter().filter(|b| **b).count();

    let mut room_lookup = vec![-1_i32; cell_count];
    let rooms = extract_rooms(&sealed_free, &outside, grid_w, grid_h, &mut room_lookup);

    let gap_edges = extract_gap_edges(
        &door_clusters,
        &free,
        &outside,
        &room_lookup,
        grid_w,
        grid_h,
    );
    let gaps = dedup_gap_points(gap_edges.iter().map(|g| g.pos));

    LevelRooms {
        rooms,
        gaps,
        gap_edges,
        outside_cells,
        grid_w,
        grid_h,
        cell_size: CELL_SIZE,
        room_lookup,
    }
}

fn idx(grid_w: i32, x: i32, y: i32) -> usize {
    (y * grid_w + x) as usize
}

fn build_blocked_grid(walls: &[Wall], grid_w: i32, grid_h: i32) -> Vec<bool> {
    let mut blocked = vec![false; (grid_w * grid_h) as usize];
    for gy in 0..grid_h {
        for gx in 0..grid_w {
            let wx = (gx as f32 + 0.5) * CELL_SIZE;
            let wy = (gy as f32 + 0.5) * CELL_SIZE;
            if walls.iter().any(|w| w.overlaps(wx, wy, px_to_tiles(1.0))) {
                blocked[idx(grid_w, gx, gy)] = true;
            }
        }
    }

    if COLLISION_CLEARANCE_CELLS <= 0 {
        return blocked;
    }

    let mut inflated = blocked.clone();
    for gy in 0..grid_h {
        for gx in 0..grid_w {
            if !blocked[idx(grid_w, gx, gy)] {
                continue;
            }
            for oy in -COLLISION_CLEARANCE_CELLS..=COLLISION_CLEARANCE_CELLS {
                for ox in -COLLISION_CLEARANCE_CELLS..=COLLISION_CLEARANCE_CELLS {
                    let nx = gx + ox;
                    let ny = gy + oy;
                    if nx >= 0 && ny >= 0 && nx < grid_w && ny < grid_h {
                        inflated[idx(grid_w, nx, ny)] = true;
                    }
                }
            }
        }
    }

    inflated
}

fn detect_door_clusters(free: &[bool], grid_w: i32, grid_h: i32) -> Vec<DoorCluster> {
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

fn should_seal_cluster(
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

    // Wider cluster => doorway is vertically narrow (walls above + below).
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
        // Taller cluster => doorway is horizontally narrow (walls left + right).
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

fn is_blocked(free: &[bool], grid_w: i32, x: i32, y: i32) -> bool {
    !free[idx(grid_w, x, y)]
}

fn blocked_depth(free: &[bool], grid_w: i32, grid_h: i32, x: i32, y: i32, dx: i32, dy: i32) -> i32 {
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

fn span(
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
        if !free[idx(grid_w, cx, cy)] {
            break;
        }
        steps += 1;
    }
    steps
}

fn flood_outside(sealed_free: &[bool], grid_w: i32, grid_h: i32) -> Vec<bool> {
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

fn extract_rooms(
    sealed_free: &[bool],
    outside: &[bool],
    grid_w: i32,
    grid_h: i32,
    room_lookup: &mut [i32],
) -> Vec<DetectedRoom> {
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
                (sum_x / area_cells as f32) * CELL_SIZE,
                (sum_y / area_cells as f32) * CELL_SIZE,
            );

            rooms.push(DetectedRoom {
                id: room_id,
                x: min_x as f32 * CELL_SIZE,
                y: min_y as f32 * CELL_SIZE,
                w: (max_x - min_x + 1) as f32 * CELL_SIZE,
                h: (max_y - min_y + 1) as f32 * CELL_SIZE,
                color: ROOM_COLORS[room_id % ROOM_COLORS.len()],
                centroid,
                area_cells,
            });
        }
    }

    rooms
}

fn extract_gap_edges(
    door_clusters: &[DoorCluster],
    free: &[bool],
    outside: &[bool],
    room_lookup: &[i32],
    grid_w: i32,
    grid_h: i32,
) -> Vec<RoomGap> {
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
        let pos = ((cx + 0.5) * CELL_SIZE, (cy + 0.5) * CELL_SIZE);

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

fn dedup_gap_points(points: impl Iterator<Item = (f32, f32)>) -> Vec<(f32, f32)> {
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

fn neighbors4(x: i32, y: i32) -> [(i32, i32); 4] {
    [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)]
}

fn neighbors8(x: i32, y: i32) -> [(i32, i32); 8] {
    [
        (x + 1, y),
        (x - 1, y),
        (x, y + 1),
        (x, y - 1),
        (x + 1, y + 1),
        (x + 1, y - 1),
        (x - 1, y + 1),
        (x - 1, y - 1),
    ]
}

#[cfg(test)]
mod tests {
    use super::LevelRooms;
    use crate::core::world::units::px_to_tiles;
    use crate::core::world::wall::Wall;

    fn p(v: f32) -> f32 {
        px_to_tiles(v)
    }

    fn w(x: f32, y: f32, w: f32, h: f32) -> Wall {
        Wall::new(p(x), p(y), p(w), p(h))
    }

    #[test]
    fn detects_single_closed_room() {
        let walls = vec![
            w(80.0, 80.0, 160.0, 16.0),
            w(80.0, 224.0, 160.0, 16.0),
            w(80.0, 80.0, 16.0, 160.0),
            w(224.0, 80.0, 16.0, 160.0),
        ];

        let rooms = LevelRooms::compute(&walls, p(400.0), p(320.0));
        assert_eq!(rooms.rooms.len(), 1);
        assert_eq!(rooms.find_room_at(p(160.0), p(160.0)), Some(0));
        assert_eq!(rooms.find_room_at(p(40.0), p(40.0)), None);
    }

    #[test]
    fn separates_rooms_through_narrow_doorway() {
        let walls = vec![
            // Outer shell
            w(64.0, 64.0, 320.0, 16.0),
            w(64.0, 224.0, 320.0, 16.0),
            w(64.0, 64.0, 16.0, 176.0),
            w(368.0, 64.0, 16.0, 176.0),
            // Center divider with a narrow opening between y=[128,160)
            w(216.0, 64.0, 16.0, 64.0),
            w(216.0, 160.0, 16.0, 80.0),
        ];

        let rooms = LevelRooms::compute(&walls, p(480.0), p(320.0));
        assert_eq!(rooms.rooms.len(), 2);

        let left = rooms.find_room_at(p(140.0), p(150.0)).expect("left room");
        let right = rooms.find_room_at(p(300.0), p(150.0)).expect("right room");
        assert_ne!(left, right);

        let linked = rooms.gap_edges.iter().any(|g| {
            let a = g.from_room;
            let b = g.to_room;
            (a == Some(left) && b == Some(right)) || (a == Some(right) && b == Some(left))
        });
        assert!(linked, "expected a doorway edge between the two rooms");
    }

    #[test]
    fn does_not_create_room_for_open_u_shape() {
        let walls = vec![
            w(96.0, 96.0, 16.0, 144.0),
            w(224.0, 96.0, 16.0, 144.0),
            w(96.0, 224.0, 144.0, 16.0),
        ];

        let rooms = LevelRooms::compute(&walls, p(400.0), p(320.0));
        assert_eq!(rooms.rooms.len(), 0);
        assert_eq!(rooms.find_room_at(p(160.0), p(180.0)), None);
    }

    #[test]
    fn near_corner_narrow_opening_is_treated_as_doorway() {
        let walls = vec![
            // Almost closed box with a top-right narrow opening.
            w(80.0, 80.0, 152.0, 16.0),
            w(80.0, 224.0, 160.0, 16.0),
            w(80.0, 80.0, 16.0, 160.0),
            w(224.0, 120.0, 16.0, 120.0),
        ];

        let rooms = LevelRooms::compute(&walls, p(400.0), p(320.0));
        assert_eq!(rooms.rooms.len(), 1);
        assert_eq!(rooms.find_room_at(p(160.0), p(160.0)), Some(0));
    }

    #[test]
    fn detects_concave_closed_room_with_wall_stub() {
        let walls = vec![
            // Outer shell.
            w(64.0, 64.0, 256.0, 16.0),
            w(64.0, 256.0, 256.0, 16.0),
            w(64.0, 64.0, 16.0, 208.0),
            w(304.0, 64.0, 16.0, 208.0),
            // Wall protruding from the right side into the room (concave, 6+ corners).
            w(224.0, 144.0, 96.0, 16.0),
        ];

        let rooms = LevelRooms::compute(&walls, p(480.0), p(320.0));
        assert_eq!(rooms.rooms.len(), 1);

        let above_stub = rooms.find_room_at(p(176.0), p(120.0)).expect("upper area");
        let below_stub = rooms.find_room_at(p(176.0), p(200.0)).expect("lower area");
        assert_eq!(above_stub, below_stub);
    }

    #[test]
    fn does_not_split_concave_room_with_deep_wall_stub() {
        let walls = vec![
            // Outer shell.
            w(64.0, 64.0, 256.0, 16.0),
            w(64.0, 256.0, 256.0, 16.0),
            w(64.0, 64.0, 16.0, 208.0),
            w(304.0, 64.0, 16.0, 208.0),
            // Deep wall stub still leaves one connected concave room.
            w(112.0, 144.0, 208.0, 16.0),
        ];

        let rooms = LevelRooms::compute(&walls, p(480.0), p(320.0));
        assert_eq!(rooms.rooms.len(), 1);

        let upper = rooms.find_room_at(p(176.0), p(120.0)).expect("upper area");
        let lower = rooms.find_room_at(p(176.0), p(200.0)).expect("lower area");
        assert_eq!(upper, lower);
    }
}
