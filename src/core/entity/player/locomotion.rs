use crate::core::entity::movement::{Movement, MovementInput};
use crate::core::world::units::px_to_tiles;
use crate::core::world::wall::Wall;
use crate::input::InputState;

const WALK_SPEED: f32 = px_to_tiles(40.0);
const NORMAL_SPEED: f32 = px_to_tiles(100.0);
pub const RUN_SPEED: f32 = px_to_tiles(280.0);
const SPRINT_BURST_SECS: f32 = 1.2;
const SPRINT_COOLDOWN_SECS: f32 = 8.0;
const PEEK_DISTANCE: f32 = px_to_tiles(18.0);

pub struct LocomotionState {
    peek_origin: Option<(f32, f32)>,
    was_peeking: bool,
    sprint_time_left: f32,
    sprint_cooldown_left: f32,
}

impl LocomotionState {
    pub fn new() -> Self {
        Self {
            peek_origin: None,
            was_peeking: false,
            sprint_time_left: SPRINT_BURST_SECS,
            sprint_cooldown_left: 0.0,
        }
    }

    pub fn step(
        &mut self,
        dt: f32,
        input: &InputState,
        movement: &mut Movement,
        half_size: f32,
        walls: &[Wall],
    ) {
        if self.sprint_cooldown_left > 0.0 {
            self.sprint_cooldown_left = (self.sprint_cooldown_left - dt).max(0.0);
            if self.sprint_cooldown_left <= 0.0 {
                self.sprint_time_left = SPRINT_BURST_SECS;
            }
        }

        let sprinting =
            input.shift && self.sprint_cooldown_left <= 0.0 && self.sprint_time_left > 0.0;
        if sprinting {
            self.sprint_time_left = (self.sprint_time_left - dt).max(0.0);
            if self.sprint_time_left <= 0.0 {
                self.sprint_cooldown_left = SPRINT_COOLDOWN_SECS;
            }
        }

        movement.speed = if sprinting {
            RUN_SPEED
        } else if input.walk {
            WALK_SPEED
        } else {
            NORMAL_SPEED
        };

        let px: f32 = if input.right {
            1.0
        } else if input.left {
            -1.0
        } else {
            0.0
        };
        let py: f32 = if input.down {
            1.0
        } else if input.up {
            -1.0
        } else {
            0.0
        };
        let has_dir = px != 0.0 || py != 0.0;
        if input.peek && self.peek_origin.is_none() {
            self.peek_origin = Some((movement.x, movement.y));
        }
        let is_peeking = input.peek && has_dir;

        if is_peeking {
            let origin = self.peek_origin.unwrap_or((movement.x, movement.y));
            let len = (px * px + py * py).sqrt();
            let dir = (px / len, py / len);
            let dist = clamped_peek_distance(origin, dir, PEEK_DISTANCE, half_size, walls);
            movement.x = origin.0 + dir.0 * dist;
            movement.y = origin.1 + dir.1 * dist;
            movement.velocity_frac = 0.0;
            self.was_peeking = true;
        } else {
            if self.was_peeking && !has_dir {
                if let Some(origin) = self.peek_origin {
                    movement.x = origin.0;
                    movement.y = origin.1;
                }
            }
            self.was_peeking = false;

            if !input.peek {
                self.peek_origin = None;
                movement.apply(
                    MovementInput {
                        up: input.up,
                        down: input.down,
                        left: input.left,
                        right: input.right,
                    },
                    dt,
                );
            } else {
                movement.velocity_frac = 0.0;
            }
        }
    }
}

fn clamped_peek_distance(
    origin: (f32, f32),
    dir: (f32, f32),
    max_dist: f32,
    half_size: f32,
    walls: &[Wall],
) -> f32 {
    const PEEK_STEP: f32 = px_to_tiles(0.5);
    let mut safe_dist = 0.0;
    let mut d = 0.0;
    while d < max_dist {
        d = (d + PEEK_STEP).min(max_dist);
        let cx = origin.0 + dir.0 * d;
        let cy = origin.1 + dir.1 * d;
        if walls.iter().any(|w| w.overlaps(cx, cy, half_size)) {
            break;
        }
        safe_dist = d;
    }
    safe_dist
}
