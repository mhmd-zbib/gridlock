use crate::input::InputState;
use super::movement::{Movement, MovementInput};
use crate::core::world::sight::Sight;
use crate::core::world::aim_cone::AimCone;
use crate::core::spawn::{SpawnQueue, SpawnRequest};

pub struct Player {
    pub movement:  Movement,
    pub sight:     Sight,
    pub aim_cone:  AimCone,
    was_shooting:  bool,
}

impl Player {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            movement:    Movement::new(x, y, 200.0),
            sight:       Sight::player(),
            aim_cone:    AimCone::new(),
            was_shooting: false,
        }
    }

    pub fn update(&mut self, dt: f32, input: &InputState, spawns: &mut SpawnQueue) {
        let mv = MovementInput {
            up:    input.up,
            down:  input.down,
            left:  input.left,
            right: input.right,
        };
        self.movement.apply(mv, dt);

        let from = (self.movement.x, self.movement.y);
        let to   = (input.mouse_x as f32, input.mouse_y as f32);

        // Sight and aim cone share the same facing direction.
        self.sight.face(from, to, dt);
        self.aim_cone.direction = self.sight.direction;

        // Advance the aim cone (decay recoil + smooth movement spread).
        self.aim_cone.update(dt, self.movement.velocity_frac);

        // Shoot once per press, direction randomised within the current aim cone.
        let just_pressed = input.shoot && !self.was_shooting;
        if just_pressed {
            let (dir_x, dir_y) = self.aim_cone.sample_direction();
            spawns.push(SpawnRequest::Bullet {
                x: from.0, y: from.1,
                dir_x, dir_y,
            });
            self.aim_cone.on_shot();
        }
        self.was_shooting = input.shoot;
    }
}
