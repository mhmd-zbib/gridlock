pub const ROOM_COLORS: [[f32; 4]; 6] = [
    [0.28, 0.52, 0.90, 0.12],
    [0.28, 0.78, 0.40, 0.12],
    [0.88, 0.44, 0.28, 0.12],
    [0.64, 0.28, 0.84, 0.12],
    [0.84, 0.72, 0.18, 0.12],
    [0.18, 0.72, 0.72, 0.12],
];

pub const OUTSIDE_REGION_ID: i32 = -1;

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

#[derive(Clone)]
pub struct DoorCluster {
    pub cells: Vec<(i32, i32)>,
    pub min_x: i32,
    pub min_y: i32,
    pub max_x: i32,
    pub max_y: i32,
}
