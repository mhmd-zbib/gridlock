use super::bullet::BulletOwner;
use super::movement::{Movement, MovementInput};
use crate::core::spawn::{SpawnQueue, SpawnRequest};
use crate::core::world::aim_cone::AimCone;
use crate::core::world::sight::Sight;
use crate::core::world::wall::Wall;
use crate::input::InputState;

const WALK_SPEED: f32 = 80.0;
const NORMAL_SPEED: f32 = 150.0;
const RUN_SPEED: f32 = 300.0;
const PEEK_DISTANCE: f32 = 18.0;

pub struct Player {
    pub movement: Movement,
    pub sight: Sight,
    pub aim_cone: AimCone,
    was_shooting: bool,
    peek_origin: Option<(f32, f32)>,
    was_peeking: bool,
}

impl Player {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            movement: Movement::new(x, y, NORMAL_SPEED),
            sight: Sight::player(),
            aim_cone: AimCone::new(),
            was_shooting: false,
            peek_origin: None,
            was_peeking: false,
        }
    }

    pub fn update(
        &mut self,
        dt: f32,
        input: &InputState,
        walls: &[Wall],
        half_size: f32,
        spawns: &mut SpawnQueue,
    ) {
        self.movement.speed = if input.shift {
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
            self.peek_origin = Some((self.movement.x, self.movement.y));
        }
        let is_peeking = input.peek && has_dir;

        if is_peeking {
            let origin = self
                .peek_origin
                .unwrap_or((self.movement.x, self.movement.y));
            let len = (px * px + py * py).sqrt();
            let dir = (px / len, py / len);
            let dist = clamped_peek_distance(origin, dir, PEEK_DISTANCE, half_size, walls);
            self.movement.x = origin.0 + dir.0 * dist;
            self.movement.y = origin.1 + dir.1 * dist;
            self.movement.velocity_frac = 0.0;
            self.was_peeking = true;
        } else {
            if self.was_peeking && !has_dir {
                if let Some(origin) = self.peek_origin {
                    self.movement.x = origin.0;
                    self.movement.y = origin.1;
                }
            }
            self.was_peeking = false;

            if !input.peek {
                self.peek_origin = None;
                let mv = MovementInput {
                    up: input.up,
                    down: input.down,
                    left: input.left,
                    right: input.right,
                };
                self.movement.apply(mv, dt);
            } else {
                self.movement.velocity_frac = 0.0;
            }
        }

        let from = (self.movement.x, self.movement.y);
        let to = (input.mouse_x as f32, input.mouse_y as f32);

        // Sight and aim cone share the same facing direction.
        self.sight.face(from, to, dt);
        self.aim_cone.direction = self.sight.direction;

        // Scale velocity fraction by speed ratio so walk = tight cone, run = wide cone.
        let speed_frac = self.movement.velocity_frac * (self.movement.speed / RUN_SPEED);
        self.aim_cone.update(dt, speed_frac);

        // Shoot once per press, direction randomised within the current aim cone.
        let just_pressed = input.shoot && !self.was_shooting;
        if just_pressed {
            let (dir_x, dir_y) = self.aim_cone.sample_direction();
            spawns.push(SpawnRequest::Bullet {
                x: from.0,
                y: from.1,
                dir_x,
                dir_y,
                owner: BulletOwner::Player,
            });
            self.aim_cone.on_shot();
        }
        self.was_shooting = input.shoot;
    }
}

fn clamped_peek_distance(
    origin: (f32, f32),
    dir: (f32, f32),
    max_dist: f32,
    half_size: f32,
    walls: &[Wall],
) -> f32 {
    // Step forward in small increments so corner/diagonal peeks can't tunnel through walls.
    const PEEK_STEP: f32 = 0.5;
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
