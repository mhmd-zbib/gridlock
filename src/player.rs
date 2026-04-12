use crate::input::InputState;
use crate::movement::{Movement, MovementInput};
use crate::spawn::{SpawnQueue, SpawnRequest};

pub struct Player {
    pub movement: Movement,
    /// Prevent holding shoot from firing every frame.
    was_shooting: bool,
}

impl Player {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            movement: Movement::new(x, y, 200.0),
            was_shooting: false,
        }
    }

    pub fn update(&mut self, dt: f32, input: &InputState, spawns: &mut SpawnQueue) {
        // --- movement ---
        let mv = MovementInput {
            up:    input.up,
            down:  input.down,
            left:  input.left,
            right: input.right,
        };
        self.movement.apply(mv, dt);

        // --- shoot: fire once per press, aimed at the mouse cursor ---
        let just_pressed = input.shoot && !self.was_shooting;
        if just_pressed {
            let dx = input.mouse_x as f32 - self.movement.x;
            let dy = input.mouse_y as f32 - self.movement.y;
            let len = (dx * dx + dy * dy).sqrt();

            if len > 0.0 {
                spawns.push(SpawnRequest::Bullet {
                    x: self.movement.x,
                    y: self.movement.y,
                    dir_x: dx / len,
                    dir_y: dy / len,
                });
            }
        }
        self.was_shooting = input.shoot;
    }
}
