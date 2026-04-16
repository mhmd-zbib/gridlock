use super::types::{GapEvaluation, GapNode, SpatialGraph};
use crate::world::ray::wrap_angle;
use crate::world::spatial::{self, SCAN_RANGE, SCAN_SECTORS};
use std::f32::consts::PI;

const BASE_ATTENTION_TIME: f32 = 0.95;
const ANCHOR_LAMBDA: f32 = 0.32;
const THREAT_MEMORY_WINDOW: f32 = 6.0;
const THREAT_BOOST: f32 = 0.22;
const DANGER_HISTORY_BOOST: f32 = 0.12;
const DIRECTION_MEMORY_BOOST: f32 = 0.08;

pub(super) const HOLD_MIN_TIME: f32 = 0.25;

const W_DISTANCE: f32 = 0.30;
const W_CONNECTIVITY: f32 = 0.20;
const W_DEPTH: f32 = 0.30;
const W_RISK: f32 = 0.20;

pub(super) fn score_gaps(
    graph: &SpatialGraph,
    enemy_pos: (f32, f32),
    suspicion: f32,
    last_known: (f32, f32),
    spawn_anchor: (f32, f32),
    threat_age: &[f32; SCAN_SECTORS],
    danger_history: &[f32; SCAN_SECTORS],
    threat_dir: &[f32; SCAN_SECTORS],
) -> Vec<GapEvaluation> {
    if graph.gaps.is_empty() {
        return Vec::new();
    }

    let max_conn = graph
        .gaps
        .iter()
        .map(|g| g.connected_rooms_count as f32)
        .fold(1.0_f32, f32::max);
    let max_depth = graph
        .gaps
        .iter()
        .map(|g| g.depth_to_other_rooms)
        .fold(1.0_f32, f32::max);
    let max_edges = graph
        .edges
        .iter()
        .filter(|e| e.from_room == graph.room.id)
        .count()
        .max(1) as f32;

    let threat_presence = suspicion.clamp(0.0, 1.0) * 1.6 + graph.room.danger_history * 0.35;
    let room_depth_norm = normalized(graph.room.depth as f32, 8.0);
    let room_size_norm = normalized(graph.room.size, SCAN_RANGE * 2.0);
    let room_gap_norm = normalized(graph.room.num_gaps as f32, max_edges);
    let room_bonus =
        (room_depth_norm * 0.05 + room_size_norm * 0.03 + room_gap_norm * 0.02).clamp(0.0, 0.1);

    let mut out = Vec::with_capacity(graph.gaps.len());
    for gap in &graph.gaps {
        let d = 1.0 - normalized(gap.distance_from_enemy, SCAN_RANGE);
        let c = normalized(gap.connected_rooms_count as f32, max_conn);
        let n = normalized(gap.depth_to_other_rooms, max_depth);
        let r = 1.0 / (1.0 + threat_presence + gap.openness);
        let edge_connectivity = graph
            .edges
            .iter()
            .filter(|e| e.gap_id == gap.id && e.to_room != e.from_room)
            .count() as f32;
        let edge_connectivity_norm = normalized(edge_connectivity, max_edges);

        let base = W_DISTANCE * d
            + W_CONNECTIVITY * c
            + W_DEPTH * n
            + W_RISK * r
            + gap.choke_score * 0.08
            + edge_connectivity_norm * 0.05
            + room_bonus;
        let anchor_penalty =
            ANCHOR_LAMBDA * normalized(spatial::distance(gap.pos, spawn_anchor), SCAN_RANGE * 1.7);
        let memory_boost = gap_memory_boost(
            gap,
            enemy_pos,
            last_known,
            threat_age,
            danger_history,
            threat_dir,
        );
        let score = (base + memory_boost - anchor_penalty).max(0.0);

        out.push(GapEvaluation {
            gap_id: gap.id,
            sector: gap.sector,
            pos: gap.pos,
            score,
            norm_score: 0.0,
            attention_time: 0.0,
        });
    }

    let max_score = out.iter().map(|g| g.score).fold(0.0_f32, f32::max);
    let base_time = BASE_ATTENTION_TIME * (1.0 + suspicion.clamp(0.0, 1.0) * 0.65);
    for g in &mut out {
        g.norm_score = if max_score > 0.0 {
            (g.score / max_score).clamp(0.0, 1.0)
        } else {
            1.0
        };
        g.attention_time = (g.norm_score * base_time).clamp(HOLD_MIN_TIME, base_time);
    }

    out
}

fn gap_memory_boost(
    gap: &GapNode,
    enemy_pos: (f32, f32),
    last_known: (f32, f32),
    threat_age: &[f32; SCAN_SECTORS],
    danger_history: &[f32; SCAN_SECTORS],
    threat_dir: &[f32; SCAN_SECTORS],
) -> f32 {
    let age = threat_age[gap.sector];
    let recency = (1.0 - age / THREAT_MEMORY_WINDOW).clamp(0.0, 1.0);
    let danger = (danger_history[gap.sector] / 2.5).clamp(0.0, 1.0);

    let gap_dir = (gap.pos.1 - enemy_pos.1).atan2(gap.pos.0 - enemy_pos.0);
    let remembered_dir = threat_dir[gap.sector];
    let last_known_dir = (last_known.1 - enemy_pos.1).atan2(last_known.0 - enemy_pos.0);
    let memory_align = 1.0 - (wrap_angle(gap_dir - remembered_dir).abs() / PI).clamp(0.0, 1.0);
    let last_known_align = 1.0 - (wrap_angle(gap_dir - last_known_dir).abs() / PI).clamp(0.0, 1.0);

    recency * THREAT_BOOST
        + danger * DANGER_HISTORY_BOOST
        + memory_align * recency * DIRECTION_MEMORY_BOOST
        + last_known_align * 0.05
}

fn normalized(v: f32, max: f32) -> f32 {
    if max <= 0.0 {
        0.0
    } else {
        (v / max).clamp(0.0, 1.0)
    }
}
