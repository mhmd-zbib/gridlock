mod flood;
mod gaps;
mod grid;
mod types;

pub use types::{DetectedRoom, RoomGap};

use crate::world::units::px_to_tiles;
use crate::world::wall::Wall;

use flood::{extract_rooms, flood_outside};
use gaps::{dedup_gap_points, detect_door_clusters, extract_gap_edges, should_seal_cluster};
use grid::build_blocked_grid;
use types::DoorCluster;

/// Grid cell size for occupancy / topology analysis (world tiles).
pub(super) const CELL_SIZE: f32 = px_to_tiles(8.0);

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

        self.rooms.iter().find_map(|room| {
            if x >= room.x && x < room.x + room.w && y >= room.y && y < room.y + room.h {
                Some(room.id)
            } else {
                None
            }
        })
    }
}

pub(super) fn idx(grid_w: i32, x: i32, y: i32) -> usize {
    (y * grid_w + x) as usize
}

pub(super) fn neighbors4(x: i32, y: i32) -> [(i32, i32); 4] {
    [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)]
}

pub(super) fn neighbors8(x: i32, y: i32) -> [(i32, i32); 8] {
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

#[cfg(test)]
mod tests {
    use super::LevelRooms;
    use crate::world::units::px_to_tiles;
    use crate::world::wall::Wall;

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
            w(64.0, 64.0, 320.0, 16.0),
            w(64.0, 224.0, 320.0, 16.0),
            w(64.0, 64.0, 16.0, 176.0),
            w(368.0, 64.0, 16.0, 176.0),
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
            w(64.0, 64.0, 256.0, 16.0),
            w(64.0, 256.0, 256.0, 16.0),
            w(64.0, 64.0, 16.0, 208.0),
            w(304.0, 64.0, 16.0, 208.0),
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
            w(64.0, 64.0, 256.0, 16.0),
            w(64.0, 256.0, 256.0, 16.0),
            w(64.0, 64.0, 16.0, 208.0),
            w(304.0, 64.0, 16.0, 208.0),
            w(112.0, 144.0, 208.0, 16.0),
        ];

        let rooms = LevelRooms::compute(&walls, p(480.0), p(320.0));
        assert_eq!(rooms.rooms.len(), 1);

        let upper = rooms.find_room_at(p(176.0), p(120.0)).expect("upper area");
        let lower = rooms.find_room_at(p(176.0), p(200.0)).expect("lower area");
        assert_eq!(upper, lower);
    }
}
