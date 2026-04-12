use super::bullet::BulletOwner;
use super::movement::{Movement, MovementInput};
use crate::core::spawn::{SpawnQueue, SpawnRequest};
use crate::core::world::aim_cone::AimCone;
use crate::core::world::sight::Sight;
use crate::input::InputState;

const WALK_SPEED: f32 = 80.0;
const NORMAL_SPEED: f32 = 150.0;
const RUN_SPEED: f32 = 300.0;

pub struct Player {
    pub movement: Movement,
    pub sight: Sight,
    pub aim_cone: AimCone,
    was_shooting: bool,
}

impl Player {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            movement: Movement::new(x, y, NORMAL_SPEED),
            sight: Sight::player(),
            aim_cone: AimCone::new(),
            was_shooting: false,
        }
    }

    pub fn update(&mut self, dt: f32, input: &InputState, spawns: &mut SpawnQueue) {
        self.movement.speed = if input.shift {
            RUN_SPEED
        } else if input.walk {
            WALK_SPEED
        } else {
            NORMAL_SPEED
        };

        let mv = MovementInput {
            up: input.up,
            down: input.down,
            left: input.left,
            right: input.right,
        };
        self.movement.apply(mv, dt);

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
