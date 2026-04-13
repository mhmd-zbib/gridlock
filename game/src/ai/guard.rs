// ─────────────────────────────────────────────────────────────────────────────
// RoomGuard
//
// Entrance-aware idle attention manager.
//
// The enemy understands its room as a set of gap nodes (openings/entrances)
// derived from a radial ray-cast.  It allocates gaze time between those gaps
// proportionally to their structural priority (openness, width, connectivity)
// with an optional directional bias from a last-known threat position.
//
// Gap detection geometry lives in `spatial`; this module owns only the
// attention-cycle state machine and the idle-specific gap scorer.
// ─────────────────────────────────────────────────────────────────────────────

use crate::world::ray::wrap_angle;
use crate::world::spatial::{self, SCAN_RANGE, SCAN_SECTORS};
use crate::world::units::px_to_tiles;
use crate::world::wall::Wall;
use std::f32::consts::PI;

const ENV_REBUILD_DIST: f32 = px_to_tiles(14.0);
/// Only track the two most important entrances.  More than two creates rapid
/// cycling that feels like jitter rather than deliberate attention.
const MAX_GAPS: usize = 2;
/// Slow, deliberate head movement — guards don't snap.
const LOOK_TURN_SPEED: f32 = 1.8; // rad/s
/// Minimum dwell on any entrance — long enough to feel settled.
const MIN_ATTENTION: f32 = 2.5; // seconds
/// Primary entrance cap — guard lingers here most of the cycle.
const MAX_ATTENTION: f32 = 7.0; // seconds
/// Base budget per gap per cycle; proportional split happens within this.
const CYCLE_BUDGET: f32 = 4.0; // seconds
/// Primary gap gets a strong preference so most time is spent on main entrance.
const PRIMARY_BOOST: f32 = 1.8;
const THREAT_DIR_BOOST: f32 = 0.18;
/// Tolerance for matching a gap to its previous position after rebuild.
const GAP_CONTINUITY_DIST: f32 = px_to_tiles(60.0);
const FALLBACK_SWEEP_RATE: f32 = 0.35; // rad/s
const FALLBACK_SWEEP_AMP: f32 = 0.20; // radians

// ─────────────────────────────────────────────────────────────────────────────

pub struct GuardDecision {
    pub look_dir: f32,
}

struct GapEntry {
    pos: (f32, f32),
    attention_time: f32,
}

pub struct RoomGuard {
    env_anchor: (f32, f32),
    env_ready: bool,
    gaps: Vec<GapEntry>,
    active_idx: usize,
    attention_timer: f32,
    look_dir: f32,
    sweep_timer: f32,
}

impl RoomGuard {
    pub fn new() -> Self {
        Self {
            env_anchor: (0.0, 0.0),
            env_ready: false,
            gaps: Vec::new(),
            active_idx: 0,
            attention_timer: 0.0,
            look_dir: 0.0,
            sweep_timer: 0.0,
        }
    }

    /// Call each frame while the enemy is in idle state.
    ///
    /// `threat_hint` — the last known player position if any.  Gaps aligned
    /// with that direction receive a score boost so the guard naturally lingers
    /// toward the most recently dangerous entrance.
    pub fn update(
        &mut self,
        pos: (f32, f32),
        threat_hint: Option<(f32, f32)>,
        walls: &[Wall],
        dt: f32,
    ) -> GuardDecision {
        let moved = spatial::distance(pos, self.env_anchor);
        if !self.env_ready || moved > ENV_REBUILD_DIST {
            self.env_anchor = pos;
            self.env_ready = true;

            let prev_pos = self.gaps.get(self.active_idx).map(|g| g.pos);
            self.rebuild_gaps(pos, threat_hint, walls);

            let found = prev_pos.and_then(|pp| {
                self.gaps
                    .iter()
                    .position(|g| spatial::distance(g.pos, pp) < GAP_CONTINUITY_DIST)
            });

            if let Some(idx) = found {
                self.active_idx = idx;
            } else {
                self.active_idx = 0;
                self.attention_timer = self.gaps.first().map(|g| g.attention_time).unwrap_or(0.0);
            }
        }

        // Fallback: no gaps detected (open area with no entrances).
        if self.gaps.is_empty() {
            self.sweep_timer += dt;
            let look = wrap_angle(
                self.look_dir + (self.sweep_timer * FALLBACK_SWEEP_RATE).sin() * FALLBACK_SWEEP_AMP,
            );
            self.look_dir = spatial::step_angle(self.look_dir, look, LOOK_TURN_SPEED * dt);
            return GuardDecision {
                look_dir: self.look_dir,
            };
        }

        self.attention_timer -= dt;
        if self.attention_timer <= 0.0 {
            self.active_idx = (self.active_idx + 1) % self.gaps.len();
            self.attention_timer = self.gaps[self.active_idx].attention_time;
        }

        let gap = &self.gaps[self.active_idx];
        let raw_look = (gap.pos.1 - pos.1).atan2(gap.pos.0 - pos.0);
        self.look_dir = spatial::step_angle(self.look_dir, raw_look, LOOK_TURN_SPEED * dt);

        GuardDecision {
            look_dir: self.look_dir,
        }
    }

    fn rebuild_gaps(&mut self, pos: (f32, f32), threat_hint: Option<(f32, f32)>, walls: &[Wall]) {
        let hits = spatial::sample_sector_hits(pos, walls);
        let candidate = spatial::detect_gap_sectors(&hits);
        let clusters = spatial::cluster_gap_sectors(&candidate);

        if clusters.is_empty() {
            self.gaps.clear();
            return;
        }

        let hint_dir: Option<f32> = threat_hint.map(|(hx, hy)| (hy - pos.1).atan2(hx - pos.0));

        let mut raw: Vec<(f32, (f32, f32))> = Vec::with_capacity(clusters.len());
        for cluster in &clusters {
            let center_idx = spatial::cluster_center_idx(cluster, &hits);
            let center_angle = spatial::sector_center_angle(center_idx);
            let dir = (center_angle.cos(), center_angle.sin());
            let cluster_hit_avg =
                cluster.iter().map(|i| hits[*i]).sum::<f32>() / cluster.len() as f32;
            let travel = (cluster_hit_avg * 0.58).clamp(px_to_tiles(44.0), SCAN_RANGE * 0.82);
            let gap_pos = (pos.0 + dir.0 * travel, pos.1 + dir.1 * travel);

            let openness = (cluster_hit_avg / SCAN_RANGE).clamp(0.0, 1.0);
            let width_norm = (cluster.len() as f32 / (SCAN_SECTORS as f32 * 0.25)).clamp(0.0, 1.0);
            let connectivity_raw = openness * 2.0 + width_norm * 1.0;
            let mut score = 0.45 * openness + 0.25 * width_norm + 0.30 * (connectivity_raw / 3.0);

            if let Some(hd) = hint_dir {
                let gap_angle = (gap_pos.1 - pos.1).atan2(gap_pos.0 - pos.0);
                let delta = wrap_angle(gap_angle - hd).abs();
                let align = (1.0 - delta / (PI * 0.5)).clamp(0.0, 1.0);
                score += align * THREAT_DIR_BOOST;
            }

            raw.push((score, gap_pos));
        }

        // Sort descending by score, then keep only the top MAX_GAPS entrances.
        raw.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        raw.truncate(MAX_GAPS);

        let total: f32 = raw.iter().map(|r| r.0).sum();
        let n = raw.len() as f32;

        self.gaps.clear();
        for (i, (score, gap_pos)) in raw.into_iter().enumerate() {
            let fraction = if total > 0.0 { score / total } else { 1.0 / n };
            let mut t = (fraction * CYCLE_BUDGET * n).clamp(MIN_ATTENTION, MAX_ATTENTION);
            if i == 0 {
                t = (t * PRIMARY_BOOST).min(MAX_ATTENTION);
            }
            self.gaps.push(GapEntry {
                pos: gap_pos,
                attention_time: t,
            });
        }
    }
}
