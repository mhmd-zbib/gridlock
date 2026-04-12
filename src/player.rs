use crate::input::InputState;
use crate::movement::{Movement, MovementInput};
use crate::sight::Sight;
use crate::spawn::{SpawnQueue, SpawnRequest};

pub struct Player {
    pub movement: Movement,
    pub sight:    Sight,
    was_shooting: bool,
}

impl Player {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            movement:     Movement::new(x, y, 200.0),
            sight:        Sight::player(),
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

        // Sight always points toward the mouse cursor.
        let from = (self.movement.x, self.movement.y);
        let to   = (input.mouse_x as f32, input.mouse_y as f32);
        self.sight.face(from, to, dt);

        // Shoot once per press, aimed at cursor.
        let just_pressed = input.shoot && !self.was_shooting;
        if just_pressed {
            let dx  = to.0 - from.0;
            let dy  = to.1 - from.1;
            let len = (dx * dx + dy * dy).sqrt();
            if len > 0.0 {
                spawns.push(SpawnRequest::Bullet {
                    x: from.0, y: from.1,
                    dir_x: dx / len,
                    dir_y: dy / len,
                });
            }
        }
        self.was_shooting = input.shoot;
    }
}
