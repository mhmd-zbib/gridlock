/// What direction an entity wants to move this frame.
/// Produced by a player controller, an AI, a replay system, etc.
/// Every entity feeds one of these into `Movement::apply`.
#[derive(Default, Clone, Copy)]
pub struct MovementInput {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
}

impl MovementInput {
    pub fn from_input(input: &engine::input::InputState) -> Self {
        Self {
            up: input.up,
            down: input.down,
            left: input.left,
            right: input.right,
        }
    }
}

/// Position + speed.  Shared by every entity that can move.
/// Knows nothing about *why* it moves — that's the caller's job.
pub struct Movement {
    pub x: f32,
    pub y: f32,
    pub speed: f32, // tiles / second

    /// Fraction of max speed this entity is currently moving at: 0.0 = still, 1.0 = full speed.
    /// Updated every call to `apply`. Diagonal movement counts as full speed (clamped to 1.0).
    pub velocity_frac: f32,
}

impl Movement {
    pub fn new(x: f32, y: f32, speed: f32) -> Self {
        Self {
            x,
            y,
            speed,
            velocity_frac: 0.0,
        }
    }

    /// Advance position by `input` over `dt` seconds and update `velocity_frac`.
    pub fn apply(&mut self, input: MovementInput, dt: f32) {
        if input.up    { self.y -= self.speed * dt; }
        if input.down  { self.y += self.speed * dt; }
        if input.left  { self.x -= self.speed * dt; }
        if input.right { self.x += self.speed * dt; }
        self.velocity_frac = if input.up || input.down || input.left || input.right { 1.0 } else { 0.0 };
    }
}
