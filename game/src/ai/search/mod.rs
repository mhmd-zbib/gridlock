mod graph;
mod memory;
mod phase;
mod scoring;
mod tick;
mod types;

use phase::Phase;
use types::{GapEvaluation, SpatialGraph};

use crate::world::spatial::{self, SCAN_RANGE, SCAN_SECTORS};
use crate::world::wall::Wall;

const ARRIVE_RADIUS: f32 = crate::world::units::px_to_tiles(28.0);
const ENV_REBUILD_DIST: f32 = crate::world::units::px_to_tiles(12.0);
const LOOK_TURN_SPEED: f32 = 4.8;
const MOVE_SCORE_THRESHOLD: f32 = 0.62;
const RETURN_FOCUS_SCORE: f32 = 0.78;
const MAX_SPAWN_DRIFT: f32 = SCAN_RANGE * 0.90;
const FOCUS_STICK_MIN_TIME: f32 = 0.75;
const FOCUS_SWITCH_SCORE_DELTA: f32 = 0.10;
const DONE_AFTER_CALM: f32 = 3.5;
const CALM_SUSPICION: f32 = 0.05;
const AI_SCAN_LOG: bool = false;

pub const EFFORT_FULL: f32 = 0.65;
pub const EFFORT_PARTIAL: f32 = 0.38;

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
        const THREAT_WINDOW: f32 = 6.0;
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
            threat_age: [THREAT_WINDOW + 1.0; SCAN_SECTORS],
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

    pub fn compute_gaps_debug(&self, pos: (f32, f32), walls: &[Wall]) -> Vec<(f32, f32)> {
        use crate::world::spatial::SCAN_RANGE;
        use crate::world::units::px_to_tiles;
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
                let travel = (cluster_hit_avg * 0.58).clamp(px_to_tiles(44.0), SCAN_RANGE * 0.82);
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

        memory::update(
            pos,
            last_known,
            suspicion,
            dt,
            &mut self.threat_age,
            &mut self.threat_dir,
            &mut self.danger_history,
            &mut self.prev_suspicion,
        );

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
                tick::tick_move_to_last_known(self, pos, last_known, suspicion, walls, dt)
            }
            Phase::Scanning => tick::tick_scanning(self, pos, last_known, suspicion, walls, dt),
            Phase::HoldingGap => {
                tick::tick_holding_gap(self, pos, last_known, suspicion, walls, dt)
            }
            Phase::ReturningToAnchor => {
                tick::tick_return_to_anchor(self, pos, last_known, suspicion, walls, dt)
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

    // ── Thin wrappers delegating to submodules ────────────────────────────────

    fn build_graph(&self, pos: (f32, f32), walls: &[Wall]) -> SpatialGraph {
        graph::build(pos, walls, self.spawn_anchor, &self.danger_history)
    }

    fn score(
        &self,
        graph: SpatialGraph,
        enemy_pos: (f32, f32),
        suspicion: f32,
        last_known: (f32, f32),
    ) -> Vec<GapEvaluation> {
        scoring::score_gaps(
            &graph,
            enemy_pos,
            suspicion,
            last_known,
            self.spawn_anchor,
            &self.threat_age,
            &self.danger_history,
            &self.threat_dir,
        )
    }

    // ── Eval helpers ──────────────────────────────────────────────────────────

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
}

fn move_threshold(suspicion: f32) -> f32 {
    let effort = if suspicion >= EFFORT_FULL {
        1.0
    } else if suspicion >= EFFORT_PARTIAL {
        0.6
    } else {
        0.25
    };
    (MOVE_SCORE_THRESHOLD - effort * 0.10).clamp(0.45, MOVE_SCORE_THRESHOLD)
}
