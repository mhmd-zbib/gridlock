use super::bullet::BulletOwner;
use super::movement::{Movement, MovementInput};
use crate::core::ai::awareness::{AiState, Awareness, Memory};
use crate::core::ai::perception::{PerceptionContext, PerceptionSystem, VisibilityFactor};
use crate::core::ai::search::SearchPlanner;
use crate::core::spawn::{SpawnQueue, SpawnRequest};
use crate::core::world::sight::Sight;
use crate::core::world::wall::Wall;

// How long between enemy shots (seconds).
const SHOOT_INTERVAL: f32 = 2.2;
// Patrol / alert cone sweep amplitude (radians).
const SWEEP_AMP: f32 = 0.75;
const AI_STATE_LOG: bool = true;
const AI_TICK_LOG_INTERVAL: f32 = 0.60;
const IDLE_RESCAN_INTERVAL: f32 = 2.6;
const IDLE_ENTRANCE_SWEEP_MOVE_TIME: f32 = 0.52;
const IDLE_ENTRANCE_SWEEP_HOLD_TIME: f32 = 0.18;
const IDLE_ENTRANCE_SWEEP_DURATION: f32 =
    IDLE_ENTRANCE_SWEEP_MOVE_TIME * 3.0 + IDLE_ENTRANCE_SWEEP_HOLD_TIME * 3.0;
const IDLE_ENTRANCE_SWEEP_AMP: f32 = SWEEP_AMP * 0.42;

const MAX_HP: u32 = 3;

pub struct Enemy {
    pub movement: Movement,
    pub sight: Sight,
    /// Set by `game.update()` — used by the renderer to cull invisible enemies.
    pub visible_to_player: bool,
    /// Suspicion level, state, and last-known-position memory.
    pub awareness: Awareness,
    pub hp: u32,

    perception: PerceptionSystem,
    shoot_timer: f32,
    sweep_timer: f32,
    base_dir: f32, // patrol base direction (set at spawn / updated while in combat)
    search: SearchPlanner,
    was_seeing: bool,
    prev_state: AiState,
    ai_log_timer: f32,
    idle_rescan_timer: f32,
    idle_lock_dir: f32,
    idle_sweep_time_left: f32,
    idle_done_logged: bool,
}

impl Enemy {
    pub fn new(x: f32, y: f32) -> Self {
        let mut perception = PerceptionSystem::new();
        perception.add_factor(VisibilityFactor);
        let sight = Sight::enemy();
        let mut awareness = Awareness::new();
        awareness.memory = Some(Memory {
            last_known_pos: (x, y),
        });
        let mut search = SearchPlanner::new();
        search.start(sight.direction);
        let idle_lock_dir = sight.direction;

        Self {
            movement: Movement::new(x, y, 90.0),
            sight,
            visible_to_player: true,
            awareness,
            hp: MAX_HP,
            perception,
            shoot_timer: SHOOT_INTERVAL,
            sweep_timer: 0.0,
            base_dir: 0.0,
            search,
            was_seeing: false,
            prev_state: AiState::Idle,
            ai_log_timer: 0.0,
            idle_rescan_timer: IDLE_RESCAN_INTERVAL,
            idle_lock_dir,
            idle_sweep_time_left: 0.0,
            idle_done_logged: false,
        }
    }

    pub fn update(
        &mut self,
        dt: f32,
        player_pos: (f32, f32),
        walls: &[Wall],
        spawns: &mut SpawnQueue,
    ) {
        let from = (self.movement.x, self.movement.y);

        // ── perception ───────────────────────────────────────────────────────
        let ctx = PerceptionContext {
            observer_pos: from,
            observer_sight: &self.sight,
            target_pos: player_pos,
            walls,
        };
        let result = self.perception.evaluate(&ctx);
        let seeing_now = result.score > 0.0;
        self.awareness.update(result.score, player_pos, dt);
        let state_now = self.awareness.state;
        self.ai_log_timer -= dt;
        self.idle_rescan_timer -= dt;

        if seeing_now {
            self.search = SearchPlanner::new();
            self.base_dir = (player_pos.1 - from.1).atan2(player_pos.0 - from.0);
            self.idle_lock_dir = self.base_dir;
            self.idle_sweep_time_left = 0.0;
            self.idle_done_logged = false;
            self.idle_rescan_timer = IDLE_RESCAN_INTERVAL;
        } else {
            let entered_alert = self.prev_state != AiState::Alert && state_now == AiState::Alert;
            let just_lost = self.was_seeing && !seeing_now;
            if (entered_alert || just_lost)
                && self.search.is_done()
                && self.awareness.last_known_pos().is_some()
            {
                if let Some(last) = self.awareness.last_known_pos() {
                    let dir = (last.1 - from.1).atan2(last.0 - from.0);
                    self.search.start(dir);
                    self.idle_done_logged = false;
                    self.idle_rescan_timer = IDLE_RESCAN_INTERVAL;
                }
            }
        }

        if AI_STATE_LOG && state_now != self.prev_state {
            println!(
                "[ai/enemy] state {:?} -> {:?} suspicion={:.2} sees={}",
                self.prev_state, state_now, self.awareness.suspicion, seeing_now
            );
        }

        // ── behaviour ────────────────────────────────────────────────────────
        match state_now {
            AiState::Combat => {
                // Lock onto what is currently known. If visual is lost while suspicion
                // is still high, keep chasing the last known point instead of cheating.
                let target = if seeing_now {
                    player_pos
                } else {
                    self.awareness.last_known_pos().unwrap_or(player_pos)
                };

                self.sight.face(from, target, dt);
                self.base_dir = self.sight.direction;

                self.move_toward(target, dt);

                // Only shoot when the player is currently perceived.
                if seeing_now {
                    self.shoot_timer -= dt;
                    if self.shoot_timer <= 0.0 {
                        self.shoot_timer = SHOOT_INTERVAL;
                        let dx = player_pos.0 - from.0;
                        let dy = player_pos.1 - from.1;
                        let len = (dx * dx + dy * dy).sqrt();
                        if len > 0.0 {
                            spawns.push(SpawnRequest::Bullet {
                                x: from.0,
                                y: from.1,
                                dir_x: dx / len,
                                dir_y: dy / len,
                                owner: BulletOwner::Enemy,
                            });
                        }
                    }
                }
            }

            AiState::Alert => {
                self.idle_done_logged = false;
                self.idle_rescan_timer = IDLE_RESCAN_INTERVAL;
                if let Some(pos) = self.awareness.last_known_pos() {
                    if !self.search.is_done() {
                        let decision =
                            self.search
                                .update(from, pos, self.awareness.suspicion, walls, dt);
                        self.sight.direction = decision.look_dir;
                        self.idle_lock_dir = decision.look_dir;
                        self.idle_sweep_time_left = 0.0;
                        if let Some(move_target) = decision.move_target {
                            self.move_toward(move_target, dt);
                        }
                    } else {
                        self.sight.face(from, pos, dt);
                        self.idle_lock_dir = self.sight.direction;
                        self.idle_sweep_time_left = 0.0;
                    }
                    if AI_STATE_LOG && self.ai_log_timer <= 0.0 {
                        println!(
                            "[ai/enemy] alert-search active suspicion={:.2} last_known=({:.1},{:.1})",
                            self.awareness.suspicion, pos.0, pos.1
                        );
                        self.ai_log_timer = AI_TICK_LOG_INTERVAL;
                    }
                } else {
                    self.sweep_timer += dt;
                    self.sight.direction =
                        self.base_dir + (self.sweep_timer * 1.8).sin() * SWEEP_AMP * 1.2;
                }
            }

            AiState::Idle => {
                // Idle search pass: if we still remember a last known position,
                // run planner at low effort while standing still.
                if let Some(pos) = self.awareness.last_known_pos() {
                    if !self.search.is_done() {
                        let decision =
                            self.search
                                .update(from, pos, self.awareness.suspicion, walls, dt);
                        self.sight.direction = decision.look_dir;
                        self.idle_lock_dir = decision.look_dir;
                        self.idle_sweep_time_left = 0.0;
                        if AI_STATE_LOG && self.ai_log_timer <= 0.0 {
                            println!(
                                "[ai/enemy] idle-search active suspicion={:.2} last_known=({:.1},{:.1})",
                                self.awareness.suspicion, pos.0, pos.1
                            );
                            self.ai_log_timer = AI_TICK_LOG_INTERVAL;
                        }
                        self.idle_done_logged = false;
                        self.idle_rescan_timer = IDLE_RESCAN_INTERVAL;
                    } else {
                        if self.idle_sweep_time_left > 0.0 {
                            self.idle_sweep_time_left = (self.idle_sweep_time_left - dt).max(0.0);
                            let elapsed = IDLE_ENTRANCE_SWEEP_DURATION - self.idle_sweep_time_left;
                            let offset = self.idle_gap_sweep_offset(elapsed);
                            self.sight.direction = self.idle_lock_dir + offset;

                            if self.idle_sweep_time_left <= 0.0 {
                                self.sight.direction = self.idle_lock_dir;
                                self.idle_rescan_timer = IDLE_RESCAN_INTERVAL;
                                if AI_STATE_LOG {
                                    println!("[ai/enemy] idle-gap sweep end; relock entrance");
                                }
                            }
                        } else {
                            self.sight.direction = self.idle_lock_dir;
                            if self.idle_rescan_timer <= 0.0 {
                                self.idle_sweep_time_left = IDLE_ENTRANCE_SWEEP_DURATION;
                                self.idle_rescan_timer = IDLE_RESCAN_INTERVAL;
                                self.idle_done_logged = false;
                                if AI_STATE_LOG {
                                    println!("[ai/enemy] idle-gap sweep start");
                                }
                            }
                        }
                        if AI_STATE_LOG && !self.idle_done_logged && self.ai_log_timer <= 0.0 {
                            println!(
                                "[ai/enemy] idle-search done; locked entrance suspicion={:.2}",
                                self.awareness.suspicion
                            );
                            self.ai_log_timer = AI_TICK_LOG_INTERVAL;
                            self.idle_done_logged = true;
                        }
                    }
                } else {
                    // Slow sweep — passive patrol.
                    self.sweep_timer += dt;
                    self.sight.direction =
                        self.base_dir + (self.sweep_timer * 0.45).sin() * SWEEP_AMP;
                    if AI_STATE_LOG && self.ai_log_timer <= 0.0 {
                        println!(
                            "[ai/enemy] idle-patrol no-memory suspicion={:.2}",
                            self.awareness.suspicion
                        );
                        self.ai_log_timer = AI_TICK_LOG_INTERVAL;
                    }
                }
            }
        }

        self.was_seeing = seeing_now;
        self.prev_state = state_now;
    }

    fn move_toward(&mut self, target: (f32, f32), dt: f32) {
        let from = (self.movement.x, self.movement.y);
        let dx = target.0 - from.0;
        let dy = target.1 - from.1;
        self.movement.apply(
            MovementInput {
                up: dy < 0.0,
                down: dy > 0.0,
                left: dx < 0.0,
                right: dx > 0.0,
            },
            dt,
        );
    }

    fn idle_gap_sweep_offset(&self, elapsed: f32) -> f32 {
        let mut t = elapsed.clamp(0.0, IDLE_ENTRANCE_SWEEP_DURATION);

        if t < IDLE_ENTRANCE_SWEEP_MOVE_TIME {
            let p = Self::smooth_step(t / IDLE_ENTRANCE_SWEEP_MOVE_TIME);
            return Self::lerp(0.0, -IDLE_ENTRANCE_SWEEP_AMP, p);
        }
        t -= IDLE_ENTRANCE_SWEEP_MOVE_TIME;

        if t < IDLE_ENTRANCE_SWEEP_HOLD_TIME {
            return -IDLE_ENTRANCE_SWEEP_AMP;
        }
        t -= IDLE_ENTRANCE_SWEEP_HOLD_TIME;

        if t < IDLE_ENTRANCE_SWEEP_MOVE_TIME {
            let p = Self::smooth_step(t / IDLE_ENTRANCE_SWEEP_MOVE_TIME);
            return Self::lerp(-IDLE_ENTRANCE_SWEEP_AMP, IDLE_ENTRANCE_SWEEP_AMP, p);
        }
        t -= IDLE_ENTRANCE_SWEEP_MOVE_TIME;

        if t < IDLE_ENTRANCE_SWEEP_HOLD_TIME {
            return IDLE_ENTRANCE_SWEEP_AMP;
        }
        t -= IDLE_ENTRANCE_SWEEP_HOLD_TIME;

        if t < IDLE_ENTRANCE_SWEEP_MOVE_TIME {
            let p = Self::smooth_step(t / IDLE_ENTRANCE_SWEEP_MOVE_TIME);
            return Self::lerp(IDLE_ENTRANCE_SWEEP_AMP, 0.0, p);
        }
        t -= IDLE_ENTRANCE_SWEEP_MOVE_TIME;

        if t < IDLE_ENTRANCE_SWEEP_HOLD_TIME {
            return 0.0;
        }
        0.0
    }

    fn smooth_step(t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }

    fn lerp(a: f32, b: f32, t: f32) -> f32 {
        a + (b - a) * t
    }
}
