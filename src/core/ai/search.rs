// ─────────────────────────────────────────────────────────────────────────────
// SearchPlanner
//
// Hierarchical spatial search:
//   1) Build a lightweight world graph (room + gap nodes + edges)
//   2) Score gap nodes using weighted spatial factors
//   3) Run a simple probabilistic attention loop over scored gaps
//
// Geometric scanning (ray casting, sector clustering) lives in `spatial`.
// This module owns only the search state machine and scoring logic.
// ─────────────────────────────────────────────────────────────────────────────

use crate::core::world::ray::wrap_angle;
use crate::core::world::spatial::{self, SCAN_RANGE, SCAN_SECTORS};
use crate::core::world::wall::Wall;
use std::f32::consts::PI;

const ARRIVE_RADIUS: f32 = 28.0;
const ENV_REBUILD_DIST: f32 = 12.0;
const LOOK_TURN_SPEED: f32 = 4.8;
const BASE_ATTENTION_TIME: f32 = 0.95;
const MOVE_SCORE_THRESHOLD: f32 = 0.62;
const RETURN_FOCUS_SCORE: f32 = 0.78;
const MAX_SPAWN_DRIFT: f32 = SCAN_RANGE * 0.90;
const HOLD_MIN_TIME: f32 = 0.25;
const FOCUS_STICK_MIN_TIME: f32 = 0.75;
const FOCUS_SWITCH_SCORE_DELTA: f32 = 0.10;
const DONE_AFTER_CALM: f32 = 3.5;
const CALM_SUSPICION: f32 = 0.05;
const THREAT_MEMORY_WINDOW: f32 = 6.0;
const THREAT_BOOST: f32 = 0.22;
const DANGER_HISTORY_BOOST: f32 = 0.12;
const DIRECTION_MEMORY_BOOST: f32 = 0.08;
const ANCHOR_LAMBDA: f32 = 0.32;
const AI_SCAN_LOG: bool = true;

const W_DISTANCE: f32 = 0.30;
const W_CONNECTIVITY: f32 = 0.20;
const W_DEPTH: f32 = 0.30;
const W_RISK: f32 = 0.20;

pub const EFFORT_FULL: f32 = 0.65;
pub const EFFORT_PARTIAL: f32 = 0.38;

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    MovingToLastKnown,
    Scanning,
    HoldingGap,
    ReturningToAnchor,
    Done,
}

#[derive(Clone, Copy)]
struct RoomNode {
    id: usize,
    size: f32,
    danger_history: f32,
    num_gaps: usize,
    depth: u8,
}

#[derive(Clone, Copy)]
struct GapNode {
    id: usize,
    sector: usize,
    pos: (f32, f32),
    distance_from_enemy: f32,
    connected_rooms_count: u8,
    depth_to_other_rooms: f32,
    choke_score: f32,
    openness: f32,
}

#[derive(Clone, Copy)]
struct SpatialEdge {
    from_room: usize,
    to_room: usize,
    gap_id: usize,
}

struct SpatialGraph {
    room: RoomNode,
    gaps: Vec<GapNode>,
    edges: Vec<SpatialEdge>,
}

#[derive(Clone, Copy)]
struct GapEvaluation {
    gap_id: usize,
    sector: usize,
    pos: (f32, f32),
    score: f32,
    norm_score: f32,
    attention_time: f32,
}

pub struct SearchDecision {
    pub move_target: Option<(f32, f32)>,
    pub look_dir: f32,
}

pub struct SearchPlanner {
    phase: Phase,
    approach_dir: f32,
    spawn_anchor: (f32, f32),
    spawn_anchor_set: bool,
    env_anchor: (f32, f32),
    env_ready: bool,

    focus_gap_id: Option<usize>,
    active_gap_id: Option<usize>,
    active_gap_timer: f32,
    hold_timer: f32,
    calm_timer: f32,

    prev_suspicion: f32,
    threat_age: [f32; SCAN_SECTORS],
    threat_dir: [f32; SCAN_SECTORS],
    danger_history: [f32; SCAN_SECTORS],
}

impl SearchPlanner {
    pub fn new() -> Self {
        Self {
            phase: Phase::Done,
            approach_dir: 0.0,
            spawn_anchor: (0.0, 0.0),
            spawn_anchor_set: false,
            env_anchor: (0.0, 0.0),
            env_ready: false,

            focus_gap_id: None,
            active_gap_id: None,
            active_gap_timer: 0.0,
            hold_timer: 0.0,
            calm_timer: 0.0,

            prev_suspicion: 0.0,
            threat_age: [THREAT_MEMORY_WINDOW + 1.0; SCAN_SECTORS],
            threat_dir: [0.0; SCAN_SECTORS],
            danger_history: [0.0; SCAN_SECTORS],
        }
    }

    pub fn start(&mut self, approach_dir: f32) {
        *self = Self::new();
        self.phase = Phase::MovingToLastKnown;
        self.approach_dir = approach_dir;
        if AI_SCAN_LOG {
            println!(
                "[ai/search] start phase=MovingToLastKnown approach_dir={:.2}",
                approach_dir
            );
        }
    }

    pub fn is_done(&self) -> bool {
        self.phase == Phase::Done
    }

    pub fn phase_name(&self) -> &'static str {
        match self.phase {
            Phase::MovingToLastKnown => "MovingToLast",
            Phase::Scanning => "Scanning",
            Phase::HoldingGap => "HoldingGap",
            Phase::ReturningToAnchor => "Returning",
            Phase::Done => "Done",
        }
    }

    pub fn spawn_anchor(&self) -> (f32, f32) {
        self.spawn_anchor
    }

    /// Compute gap waypoint positions the same way `build_spatial_graph` does,
    /// for use by the debug renderer (no state is mutated).
    pub fn compute_gaps_debug(&self, pos: (f32, f32), walls: &[Wall]) -> Vec<(f32, f32)> {
        let hits = spatial::sample_sector_hits(pos, walls);
        let candidate = spatial::detect_gap_sectors(&hits);
        let clusters = spatial::cluster_gap_sectors(&candidate);
        clusters
            .iter()
            .map(|cluster| {
                let center_idx = spatial::cluster_center_idx(cluster, &hits);
                let center_angle = spatial::sector_center_angle(center_idx);
                let dir = (center_angle.cos(), center_angle.sin());
                let cluster_hit_avg =
                    cluster.iter().map(|i| hits[*i]).sum::<f32>() / cluster.len() as f32;
                let travel = (cluster_hit_avg * 0.58).clamp(44.0, SCAN_RANGE * 0.82);
                (pos.0 + dir.0 * travel, pos.1 + dir.1 * travel)
            })
            .collect()
    }

    pub fn update(
        &mut self,
        pos: (f32, f32),
        last_known: (f32, f32),
        suspicion: f32,
        walls: &[Wall],
        dt: f32,
    ) -> SearchDecision {
        if !self.spawn_anchor_set {
            self.spawn_anchor = pos;
            self.spawn_anchor_set = true;
        }

        self.update_gap_memory(pos, last_known, suspicion, dt);

        if suspicion <= CALM_SUSPICION {
            self.calm_timer += dt;
        } else {
            self.calm_timer = 0.0;
        }

        if self.phase == Phase::Done && suspicion >= CALM_SUSPICION * 2.0 {
            self.phase = Phase::Scanning;
        }

        if self.calm_timer >= DONE_AFTER_CALM
            && spatial::distance(pos, self.spawn_anchor) <= ARRIVE_RADIUS
            && self.phase != Phase::MovingToLastKnown
        {
            self.phase = Phase::Done;
        }

        let decision = match self.phase {
            Phase::MovingToLastKnown => {
                self.tick_move_to_last_known(pos, last_known, suspicion, walls, dt)
            }
            Phase::Scanning => self.tick_scanning(pos, last_known, suspicion, walls, dt),
            Phase::HoldingGap => self.tick_holding_gap(pos, last_known, suspicion, walls, dt),
            Phase::ReturningToAnchor => {
                self.tick_return_to_anchor(pos, last_known, suspicion, walls, dt)
            }
            Phase::Done => SearchDecision {
                move_target: None,
                look_dir: self.approach_dir,
            },
        };

        let smooth_look =
            spatial::step_angle(self.approach_dir, decision.look_dir, LOOK_TURN_SPEED * dt);
        self.approach_dir = smooth_look;
        SearchDecision {
            move_target: decision.move_target,
            look_dir: smooth_look,
        }
    }

    fn tick_move_to_last_known(
        &mut self,
        pos: (f32, f32),
        last_known: (f32, f32),
        suspicion: f32,
        walls: &[Wall],
        dt: f32,
    ) -> SearchDecision {
        let dx = last_known.0 - pos.0;
        let dy = last_known.1 - pos.1;
        let dist = (dx * dx + dy * dy).sqrt();
        if dist <= ARRIVE_RADIUS {
            self.phase = Phase::Scanning;
            return self.tick_scanning(pos, last_known, suspicion, walls, dt);
        }
        SearchDecision {
            move_target: Some(last_known),
            look_dir: dy.atan2(dx),
        }
    }

    fn tick_scanning(
        &mut self,
        pos: (f32, f32),
        last_known: (f32, f32),
        suspicion: f32,
        walls: &[Wall],
        dt: f32,
    ) -> SearchDecision {
        let moved_from_env_anchor = spatial::distance(pos, self.env_anchor);
        if !self.env_ready || moved_from_env_anchor > ENV_REBUILD_DIST {
            self.env_anchor = pos;
            self.env_ready = true;
            self.focus_gap_id = None;
            self.active_gap_id = None;
            self.active_gap_timer = 0.0;
        }

        let graph = self.build_spatial_graph(pos, walls);
        let mut evals = self.score_gaps(&graph, pos, suspicion, last_known);

        if evals.is_empty() {
            if spatial::distance(pos, self.spawn_anchor) > MAX_SPAWN_DRIFT * 0.7 {
                self.phase = Phase::ReturningToAnchor;
                return self.tick_return_to_anchor(pos, last_known, suspicion, walls, dt);
            }
            self.active_gap_timer += dt;
            let look = wrap_angle(self.approach_dir + (self.active_gap_timer * 2.2).sin() * 0.25);
            return SearchDecision {
                move_target: None,
                look_dir: look,
            };
        }

        evals.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let focus = evals[0];
        self.focus_gap_id = Some(focus.gap_id);

        if !self.eval_contains_active(&evals) {
            self.active_gap_id = Some(focus.gap_id);
            self.active_gap_timer = focus.attention_time.max(FOCUS_STICK_MIN_TIME);
        } else {
            self.active_gap_timer -= dt;
            let active = self
                .find_eval(&evals, self.active_gap_id)
                .copied()
                .unwrap_or(focus);

            let can_switch = self.active_gap_timer <= 0.0;
            let focus_is_better = focus.score >= active.score + FOCUS_SWITCH_SCORE_DELTA;
            if can_switch && active.gap_id != focus.gap_id && focus_is_better {
                self.active_gap_id = Some(focus.gap_id);
                self.active_gap_timer = focus.attention_time.max(FOCUS_STICK_MIN_TIME);
            } else if can_switch && active.gap_id == focus.gap_id {
                self.active_gap_timer = active.attention_time.max(FOCUS_STICK_MIN_TIME);
            }
        }
        let current = self
            .find_eval(&evals, self.active_gap_id)
            .unwrap_or(&evals[0]);
        let look_dir = (current.pos.1 - pos.1).atan2(current.pos.0 - pos.0);

        if AI_SCAN_LOG {
            println!(
                "[ai/search] scan focus_gap={} active_gap={} sector={} score={:.2} norm={:.2} attn={:.2}s suspicion={:.2} room(depth={},size={:.1},gaps={})",
                self.focus_gap_id.unwrap_or(current.gap_id),
                current.gap_id,
                current.sector,
                current.score,
                current.norm_score,
                current.attention_time,
                suspicion,
                graph.room.depth,
                graph.room.size,
                graph.room.num_gaps
            );
        }

        let threshold = self.move_threshold(suspicion);
        if current.score >= threshold {
            let gap_dist = spatial::distance(pos, current.pos);
            if gap_dist > ARRIVE_RADIUS {
                self.phase = Phase::HoldingGap;
                self.hold_timer = current.attention_time.max(HOLD_MIN_TIME);
                return SearchDecision {
                    move_target: Some(current.pos),
                    look_dir,
                };
            }
            self.phase = Phase::HoldingGap;
            self.hold_timer = current.attention_time.max(HOLD_MIN_TIME);
        }

        if spatial::distance(pos, self.spawn_anchor) > MAX_SPAWN_DRIFT
            && current.score < RETURN_FOCUS_SCORE
        {
            self.phase = Phase::ReturningToAnchor;
            return self.tick_return_to_anchor(pos, last_known, suspicion, walls, dt);
        }

        SearchDecision {
            move_target: None,
            look_dir,
        }
    }

    fn tick_holding_gap(
        &mut self,
        pos: (f32, f32),
        last_known: (f32, f32),
        suspicion: f32,
        walls: &[Wall],
        dt: f32,
    ) -> SearchDecision {
        let graph = self.build_spatial_graph(pos, walls);
        let evals = self.score_gaps(&graph, pos, suspicion, last_known);
        let Some(current) = self.find_eval(&evals, self.active_gap_id) else {
            self.phase = Phase::Scanning;
            return self.tick_scanning(pos, last_known, suspicion, walls, dt);
        };

        let look = (current.pos.1 - pos.1).atan2(current.pos.0 - pos.0);
        let threshold = self.move_threshold(suspicion);
        let dist = spatial::distance(pos, current.pos);

        if dist > ARRIVE_RADIUS && current.score >= threshold {
            return SearchDecision {
                move_target: Some(current.pos),
                look_dir: look,
            };
        }

        self.hold_timer -= dt;
        if self.hold_timer <= 0.0 || current.score < threshold * 0.85 {
            self.phase = Phase::Scanning;
        }

        if spatial::distance(pos, self.spawn_anchor) > MAX_SPAWN_DRIFT
            && current.score < RETURN_FOCUS_SCORE
        {
            self.phase = Phase::ReturningToAnchor;
        }

        SearchDecision {
            move_target: None,
            look_dir: look,
        }
    }

    fn tick_return_to_anchor(
        &mut self,
        pos: (f32, f32),
        last_known: (f32, f32),
        suspicion: f32,
        walls: &[Wall],
        dt: f32,
    ) -> SearchDecision {
        let anchor_dx = self.spawn_anchor.0 - pos.0;
        let anchor_dy = self.spawn_anchor.1 - pos.1;
        let anchor_dist = (anchor_dx * anchor_dx + anchor_dy * anchor_dy).sqrt();

        if anchor_dist <= ARRIVE_RADIUS {
            self.phase = if suspicion <= CALM_SUSPICION {
                Phase::Done
            } else {
                Phase::Scanning
            };
            if self.phase == Phase::Done {
                return SearchDecision {
                    move_target: None,
                    look_dir: self.approach_dir,
                };
            }
            return self.tick_scanning(pos, last_known, suspicion, walls, dt);
        }

        let graph = self.build_spatial_graph(pos, walls);
        let evals = self.score_gaps(&graph, pos, suspicion, last_known);
        let look = if let Some(top) = evals.first() {
            (top.pos.1 - pos.1).atan2(top.pos.0 - pos.0)
        } else {
            anchor_dy.atan2(anchor_dx)
        };

        SearchDecision {
            move_target: Some(self.spawn_anchor),
            look_dir: look,
        }
    }

    fn build_spatial_graph(&self, pos: (f32, f32), walls: &[Wall]) -> SpatialGraph {
        let hits = spatial::sample_sector_hits(pos, walls);
        let candidate = spatial::detect_gap_sectors(&hits);
        let clusters = spatial::cluster_gap_sectors(&candidate);

        let avg_hit = hits.iter().sum::<f32>() / SCAN_SECTORS as f32;
        let room_depth = self
            .normalized(spatial::distance(pos, self.spawn_anchor), SCAN_RANGE * 1.4)
            .mul_add(6.0, 0.0)
            .round() as u8;
        let room_danger = self.danger_history.iter().sum::<f32>() / SCAN_SECTORS as f32;
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
            let cluster_hit_avg =
                cluster.iter().map(|i| hits[*i]).sum::<f32>() / cluster.len() as f32;
            let travel = (cluster_hit_avg * 0.58).clamp(44.0, SCAN_RANGE * 0.82);
            let gap_pos = (pos.0 + dir.0 * travel, pos.1 + dir.1 * travel);
            let openness = self.normalized(cluster_hit_avg, SCAN_RANGE);
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

    fn score_gaps(
        &mut self,
        graph: &SpatialGraph,
        enemy_pos: (f32, f32),
        suspicion: f32,
        last_known: (f32, f32),
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
        let room_depth_norm = self.normalized(graph.room.depth as f32, 8.0);
        let room_size_norm = self.normalized(graph.room.size, SCAN_RANGE * 2.0);
        let room_gap_norm = self.normalized(graph.room.num_gaps as f32, max_edges);
        let room_bonus =
            (room_depth_norm * 0.05 + room_size_norm * 0.03 + room_gap_norm * 0.02).clamp(0.0, 0.1);

        let mut out = Vec::with_capacity(graph.gaps.len());
        for gap in &graph.gaps {
            let d = 1.0 - self.normalized(gap.distance_from_enemy, SCAN_RANGE);
            let c = self.normalized(gap.connected_rooms_count as f32, max_conn);
            let n = self.normalized(gap.depth_to_other_rooms, max_depth);
            let r = 1.0 / (1.0 + threat_presence + gap.openness);
            let edge_connectivity = graph
                .edges
                .iter()
                .filter(|e| e.gap_id == gap.id && e.to_room != e.from_room)
                .count() as f32;
            let edge_connectivity_norm = self.normalized(edge_connectivity, max_edges);

            let base = W_DISTANCE * d
                + W_CONNECTIVITY * c
                + W_DEPTH * n
                + W_RISK * r
                + gap.choke_score * 0.08
                + edge_connectivity_norm * 0.05
                + room_bonus;
            let anchor_penalty = ANCHOR_LAMBDA
                * self.normalized(
                    spatial::distance(gap.pos, self.spawn_anchor),
                    SCAN_RANGE * 1.7,
                );
            let memory_boost = self.gap_memory_boost(gap, enemy_pos, last_known);
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
        &self,
        gap: &GapNode,
        enemy_pos: (f32, f32),
        last_known: (f32, f32),
    ) -> f32 {
        let age = self.threat_age[gap.sector];
        let recency = (1.0 - age / THREAT_MEMORY_WINDOW).clamp(0.0, 1.0);
        let danger = (self.danger_history[gap.sector] / 2.5).clamp(0.0, 1.0);

        let gap_dir = (gap.pos.1 - enemy_pos.1).atan2(gap.pos.0 - enemy_pos.0);
        let remembered_dir = self.threat_dir[gap.sector];
        let last_known_dir = (last_known.1 - enemy_pos.1).atan2(last_known.0 - enemy_pos.0);
        let memory_align = 1.0 - (wrap_angle(gap_dir - remembered_dir).abs() / PI).clamp(0.0, 1.0);
        let last_known_align =
            1.0 - (wrap_angle(gap_dir - last_known_dir).abs() / PI).clamp(0.0, 1.0);

        recency * THREAT_BOOST
            + danger * DANGER_HISTORY_BOOST
            + memory_align * recency * DIRECTION_MEMORY_BOOST
            + last_known_align * 0.05
    }

    fn update_gap_memory(
        &mut self,
        pos: (f32, f32),
        last_known: (f32, f32),
        suspicion: f32,
        dt: f32,
    ) {
        for i in 0..SCAN_SECTORS {
            self.threat_age[i] += dt;
            self.danger_history[i] = (self.danger_history[i] - dt * 0.06).max(0.0);
        }

        if suspicion > self.prev_suspicion + 0.012 {
            let dir = (last_known.1 - pos.1).atan2(last_known.0 - pos.0);
            let center = spatial::angle_to_sector(dir);
            let gain = 0.38 + suspicion.clamp(0.0, 1.0) * 0.35;
            self.register_threat(center, dir, gain);
            self.register_threat((center + 1) % SCAN_SECTORS, dir, gain * 0.42);
            self.register_threat((center + SCAN_SECTORS - 1) % SCAN_SECTORS, dir, gain * 0.42);
        }

        self.prev_suspicion = suspicion;
    }

    fn register_threat(&mut self, sector: usize, dir: f32, gain: f32) {
        self.threat_age[sector] = 0.0;
        self.threat_dir[sector] = dir;
        self.danger_history[sector] = (self.danger_history[sector] + gain).min(2.5);
    }

    fn eval_contains_active(&self, evals: &[GapEvaluation]) -> bool {
        self.find_eval(evals, self.active_gap_id).is_some()
    }

    fn find_eval<'a>(
        &self,
        evals: &'a [GapEvaluation],
        gap_id: Option<usize>,
    ) -> Option<&'a GapEvaluation> {
        let id = gap_id?;
        evals.iter().find(|e| e.gap_id == id)
    }

    // ── local math utilities ─────────────────────────────────────────────────

    fn normalized(&self, v: f32, max: f32) -> f32 {
        if max <= 0.0 {
            0.0
        } else {
            (v / max).clamp(0.0, 1.0)
        }
    }

    fn move_threshold(&self, suspicion: f32) -> f32 {
        let effort = if suspicion >= EFFORT_FULL {
            1.0
        } else if suspicion >= EFFORT_PARTIAL {
            0.6
        } else {
            0.25
        };
        (MOVE_SCORE_THRESHOLD - effort * 0.10).clamp(0.45, MOVE_SCORE_THRESHOLD)
    }
}
