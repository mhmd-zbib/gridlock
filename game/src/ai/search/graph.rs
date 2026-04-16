use super::types::{GapNode, RoomNode, SpatialEdge, SpatialGraph};
use crate::world::spatial::{self, SCAN_RANGE, SCAN_SECTORS};
use crate::world::units::px_to_tiles;
use crate::world::wall::Wall;

pub(super) fn build(
    pos: (f32, f32),
    walls: &[Wall],
    spawn_anchor: (f32, f32),
    danger_history: &[f32; SCAN_SECTORS],
) -> SpatialGraph {
    let hits = spatial::sample_sector_hits(pos, walls);
    let candidate = spatial::detect_gap_sectors(&hits);
    let clusters = spatial::cluster_gap_sectors(&candidate);

    let avg_hit = hits.iter().sum::<f32>() / SCAN_SECTORS as f32;
    let room_depth = normalized(spatial::distance(pos, spawn_anchor), SCAN_RANGE * 1.4)
        .mul_add(6.0, 0.0)
        .round() as u8;
    let room_danger = danger_history.iter().sum::<f32>() / SCAN_SECTORS as f32;
    let room = RoomNode {
        id: 0,
        size: avg_hit * 2.0,
        danger_history: room_danger,
        num_gaps: clusters.len(),
        depth: room_depth.min(8),
    };

    let mut gaps = Vec::with_capacity(clusters.len());
    let mut edges = Vec::with_capacity(clusters.len());
    for (id, cluster) in clusters.iter().enumerate() {
        let center_idx = spatial::cluster_center_idx(cluster, &hits);
        let center_angle = spatial::sector_center_angle(center_idx);
        let dir = (center_angle.cos(), center_angle.sin());
        let cluster_hit_avg = cluster.iter().map(|i| hits[*i]).sum::<f32>() / cluster.len() as f32;
        let travel = (cluster_hit_avg * 0.58).clamp(px_to_tiles(44.0), SCAN_RANGE * 0.82);
        let gap_pos = (pos.0 + dir.0 * travel, pos.1 + dir.1 * travel);
        let openness = normalized(cluster_hit_avg, SCAN_RANGE);
        let width_norm = (cluster.len() as f32 / (SCAN_SECTORS as f32 * 0.25)).clamp(0.0, 1.0);
        let choke = 1.0 - width_norm;
        let mut connected = 1 + (openness * 2.2).round() as u8;
        if cluster.len() >= 3 {
            connected = connected.saturating_add(1);
        }
        connected = connected.clamp(1, 4);

        let depth = 1.0 + openness * 3.0 + connected as f32 * 0.45;
        gaps.push(GapNode {
            id,
            sector: center_idx,
            pos: gap_pos,
            distance_from_enemy: spatial::distance(pos, gap_pos),
            connected_rooms_count: connected,
            depth_to_other_rooms: depth,
            choke_score: choke,
            openness,
        });
        edges.push(SpatialEdge {
            from_room: room.id,
            to_room: id + 1,
            gap_id: id,
        });
    }

    SpatialGraph { room, gaps, edges }
}

fn normalized(v: f32, max: f32) -> f32 {
    if max <= 0.0 {
        0.0
    } else {
        (v / max).clamp(0.0, 1.0)
    }
}
