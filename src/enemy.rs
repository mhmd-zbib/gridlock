use crate::movement::{Movement, MovementInput};

pub struct Enemy {
    pub movement: Movement,
}

impl Enemy {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            movement: Movement::new(x, y, 100.0),
        }
    }

    /// Chase `target` (typically the player's position).
    /// Converts the direction toward the target into a `MovementInput`
    /// and applies the shared movement logic.
    pub fn update(&mut self, dt: f32, target: (f32, f32)) {
        let dx = target.0 - self.movement.x;
        let dy = target.1 - self.movement.y;

        let mv = MovementInput {
            up:    dy < 0.0,
            down:  dy > 0.0,
            left:  dx < 0.0,
            right: dx > 0.0,
        };
        self.movement.apply(mv, dt);
    }
}
