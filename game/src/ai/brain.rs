// ─────────────────────────────────────────────────────────────────────────────
// EnemyBrain
//
// All enemy cognition in one place: perception, suspicion state machine, and
// the Combat / Alert / Idle behaviour implementations.
//
// The brain is intentionally decoupled from the entity (`Enemy`).  It receives
// the enemy's current position and sight as inputs, and returns a `BrainOutput`
// describing what the enemy should do — look where, move where, shoot or not.
// The entity applies those instructions; the brain never touches movement or
// rendering directly.
// ─────────────────────────────────────────────────────────────────────────────

use crate::ai::awareness::{AiState, Awareness, Memory};
use crate::ai::guard::RoomGuard;
use crate::ai::perception::{PerceptionContext, PerceptionSystem, VisibilityFactor};
use crate::ai::search::SearchPlanner;
use crate::world::sight::Sight;
use crate::world::spatial;
use crate::world::wall::Wall;

const SHOOT_INTERVAL: f32 = 2.2;
const SWEEP_AMP: f32 = 0.75;
const AI_STATE_LOG: bool = true;
const AI_TICK_LOG_INTERVAL: f32 = 0.60;

// ─────────────────────────────────────────────────────────────────────────────

/// Instructions the brain produces each frame for the entity to execute.
pub struct BrainOutput {
    /// The direction the entity should be facing.
    pub look_dir: f32,
    /// Position to move toward this frame, if any.
    pub move_target: Option<(f32, f32)>,
    /// Normalised shoot direction if the enemy should fire this frame.
    pub shoot: Option<(f32, f32)>,
}

// ─────────────────────────────────────────────────────────────────────────────

pub struct EnemyBrain {
    /// Exposed so the renderer can read suspicion state for cone colour.
    pub awareness: Awareness,
    /// Last move target produced by the brain; None when stationary. Debug use.
    pub last_move_target: Option<(f32, f32)>,
    perception: PerceptionSystem,
    search: SearchPlanner,
    room_guard: RoomGuard,
    shoot_timer: f32,
    sweep_timer: f32,
    /// Patrol / fallback facing direction; updated whenever the enemy sees the player.
    base_dir: f32,
    was_seeing: bool,
    prev_state: AiState,
    ai_log_timer: f32,
}

impl EnemyBrain {
    pub fn new(spawn_pos: (f32, f32), spawn_dir: f32) -> Self {
        let mut perception = PerceptionSystem::new();
        perception.add_factor(VisibilityFactor);

        let mut awareness = Awareness::new();
        awareness.memory = Some(Memory {
            last_known_pos: spawn_pos,
        });

        let mut search = SearchPlanner::new();
        search.start(spawn_dir);

        Self {
            awareness,
            last_move_target: None,
            perception,
            search,
            room_guard: RoomGuard::new(),
            shoot_timer: SHOOT_INTERVAL,
            sweep_timer: 0.0,
            base_dir: 0.0,
            was_seeing: false,
            prev_state: AiState::Idle,
            ai_log_timer: 0.0,
        }
    }

    /// Advance AI one frame.  Returns what the enemy should do.
    ///
    /// `sight` is read-only — the brain uses it for perception (cone direction,
    /// range) but does not mutate it.  The caller applies `output.look_dir` to
    /// `sight.direction` after this call.
    pub fn update(
        &mut self,
        pos: (f32, f32),
        sight: &Sight,
        player_pos: (f32, f32),
        walls: &[Wall],
        dt: f32,
    ) -> BrainOutput {
        // ── perception ───────────────────────────────────────────────────────
        let ctx = PerceptionContext {
            observer_pos: pos,
            observer_sight: sight,
            target_pos: player_pos,
            walls,
        };
        let result = self.perception.evaluate(&ctx);
        let seeing_now = result.score > 0.0;
        self.awareness.update(result.score, player_pos, dt);
        let state_now = self.awareness.state;
        self.ai_log_timer -= dt;

        if seeing_now {
            self.search = SearchPlanner::new();
            self.base_dir = (player_pos.1 - pos.1).atan2(player_pos.0 - pos.0);
        } else {
            let entered_alert = self.prev_state != AiState::Alert && state_now == AiState::Alert;
            let just_lost = self.was_seeing && !seeing_now;
            if (entered_alert || just_lost)
                && self.search.is_done()
                && self.awareness.last_known_pos().is_some()
            {
                if let Some(last) = self.awareness.last_known_pos() {
                    let dir = (last.1 - pos.1).atan2(last.0 - pos.0);
                    self.search.start(dir);
                }
            }
        }

        if AI_STATE_LOG && state_now != self.prev_state {
            println!(
                "[ai/brain] state {:?} -> {:?}  suspicion={:.2}  sees={}",
                self.prev_state, state_now, self.awareness.suspicion, seeing_now
            );
        }

        // ── behaviour ────────────────────────────────────────────────────────
        let output = match state_now {
            AiState::Combat => self.tick_combat(pos, sight, player_pos, seeing_now, dt),
            AiState::Alert => self.tick_alert(pos, sight, walls, dt),
            AiState::Idle => self.tick_idle(pos, walls, dt),
        };

        self.last_move_target = output.move_target;
        self.was_seeing = seeing_now;
        self.prev_state = state_now;
        output
    }

    /// Current search-phase label for debug display.
    pub fn phase_name(&self) -> &'static str {
        self.search.phase_name()
    }

    /// Spawn/anchor position the search planner is homing toward. Debug use.
    pub fn spawn_anchor(&self) -> (f32, f32) {
        self.search.spawn_anchor()
    }

    /// Re-compute gap waypoints from `pos` for debug rendering. Cheap O(36).
    pub fn debug_gaps(&self, pos: (f32, f32), walls: &[Wall]) -> Vec<(f32, f32)> {
        self.search.compute_gaps_debug(pos, walls)
    }

    // ── Combat ───────────────────────────────────────────────────────────────

    fn tick_combat(
        &mut self,
        pos: (f32, f32),
        sight: &Sight,
        player_pos: (f32, f32),
        seeing_now: bool,
        dt: f32,
    ) -> BrainOutput {
        let target = if seeing_now {
            player_pos
        } else {
            self.awareness.last_known_pos().unwrap_or(player_pos)
        };

        let target_dir = (target.1 - pos.1).atan2(target.0 - pos.0);
        let look_dir = spatial::step_angle(sight.direction, target_dir, sight.turn_speed * dt);
        self.base_dir = look_dir;

        let shoot = if seeing_now {
            self.shoot_timer -= dt;
            if self.shoot_timer <= 0.0 {
                self.shoot_timer = SHOOT_INTERVAL;
                let dx = player_pos.0 - pos.0;
                let dy = player_pos.1 - pos.1;
                let len = dx.hypot(dy);
                if len > 0.0 {
                    Some((dx / len, dy / len))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        BrainOutput {
            look_dir,
            move_target: Some(target),
            shoot,
        }
    }

    // ── Alert ────────────────────────────────────────────────────────────────

    fn tick_alert(
        &mut self,
        pos: (f32, f32),
        sight: &Sight,
        walls: &[Wall],
        dt: f32,
    ) -> BrainOutput {
        if let Some(last_known) = self.awareness.last_known_pos() {
            if !self.search.is_done() {
                let decision =
                    self.search
                        .update(pos, last_known, self.awareness.suspicion, walls, dt);
                if AI_STATE_LOG && self.ai_log_timer <= 0.0 {
                    println!(
                        "[ai/brain] alert-search  suspicion={:.2}  last_known=({:.1},{:.1})",
                        self.awareness.suspicion, last_known.0, last_known.1
                    );
                    self.ai_log_timer = AI_TICK_LOG_INTERVAL;
                }
                BrainOutput {
                    look_dir: decision.look_dir,
                    move_target: decision.move_target,
                    shoot: None,
                }
            } else {
                let target_dir = (last_known.1 - pos.1).atan2(last_known.0 - pos.0);
                let look_dir =
                    spatial::step_angle(sight.direction, target_dir, sight.turn_speed * dt);
                BrainOutput {
                    look_dir,
                    move_target: None,
                    shoot: None,
                }
            }
        } else {
            self.sweep_timer += dt;
            let look_dir = self.base_dir + (self.sweep_timer * 1.8).sin() * SWEEP_AMP * 1.2;
            BrainOutput {
                look_dir,
                move_target: None,
                shoot: None,
            }
        }
    }

    // ── Idle ─────────────────────────────────────────────────────────────────

    fn tick_idle(&mut self, pos: (f32, f32), walls: &[Wall], dt: f32) -> BrainOutput {
        let hint = self.awareness.last_known_pos();
        if let Some(last_known) = hint {
            if !self.search.is_done() {
                // Search wind-down — SearchPlanner still navigating toward gaps.
                let decision =
                    self.search
                        .update(pos, last_known, self.awareness.suspicion, walls, dt);
                if AI_STATE_LOG && self.ai_log_timer <= 0.0 {
                    println!(
                        "[ai/brain] idle-search  suspicion={:.2}  last_known=({:.1},{:.1})",
                        self.awareness.suspicion, last_known.0, last_known.1
                    );
                    self.ai_log_timer = AI_TICK_LOG_INTERVAL;
                }
                BrainOutput {
                    look_dir: decision.look_dir,
                    move_target: decision.move_target,
                    shoot: None,
                }
            } else {
                // Search complete — RoomGuard takes over, biased toward last known.
                let d = self.room_guard.update(pos, Some(last_known), walls, dt);
                if AI_STATE_LOG && self.ai_log_timer <= 0.0 {
                    println!(
                        "[ai/brain] idle-guard  suspicion={:.2}  hint=({:.1},{:.1})",
                        self.awareness.suspicion, last_known.0, last_known.1
                    );
                    self.ai_log_timer = AI_TICK_LOG_INTERVAL;
                }
                BrainOutput {
                    look_dir: d.look_dir,
                    move_target: None,
                    shoot: None,
                }
            }
        } else {
            // Never seen the player — pure structural entrance guarding.
            let d = self.room_guard.update(pos, None, walls, dt);
            if AI_STATE_LOG && self.ai_log_timer <= 0.0 {
                println!(
                    "[ai/brain] idle-guard no-memory  suspicion={:.2}",
                    self.awareness.suspicion
                );
                self.ai_log_timer = AI_TICK_LOG_INTERVAL;
            }
            BrainOutput {
                look_dir: d.look_dir,
                move_target: None,
                shoot: None,
            }
        }
    }
}
