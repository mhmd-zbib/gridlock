#[derive(Clone, Copy)]
pub struct RoomNode {
    pub id: usize,
    pub size: f32,
    pub danger_history: f32,
    pub num_gaps: usize,
    pub depth: u8,
}

#[derive(Clone, Copy)]
pub struct GapNode {
    pub id: usize,
    pub sector: usize,
    pub pos: (f32, f32),
    pub distance_from_enemy: f32,
    pub connected_rooms_count: u8,
    pub depth_to_other_rooms: f32,
    pub choke_score: f32,
    pub openness: f32,
}

#[derive(Clone, Copy)]
pub struct SpatialEdge {
    pub from_room: usize,
    pub to_room: usize,
    pub gap_id: usize,
}

pub struct SpatialGraph {
    pub room: RoomNode,
    pub gaps: Vec<GapNode>,
    pub edges: Vec<SpatialEdge>,
}

#[derive(Clone, Copy)]
pub struct GapEvaluation {
    pub gap_id: usize,
    pub sector: usize,
    pub pos: (f32, f32),
    pub score: f32,
    pub norm_score: f32,
    pub attention_time: f32,
}
