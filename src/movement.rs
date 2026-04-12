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

/// Position + speed.  Shared by every entity that can move.
/// Knows nothing about *why* it moves — that's the caller's job.
pub struct Movement {
    pub x: f32,
    pub y: f32,
    pub speed: f32, // pixels / second
}

impl Movement {
    pub fn new(x: f32, y: f32, speed: f32) -> Self {
        Self { x, y, speed }
    }

    /// Advance position by `input` over `dt` seconds.
    pub fn apply(&mut self, input: MovementInput, dt: f32) {
        if input.up    { self.y -= self.speed * dt; }
        if input.down  { self.y += self.speed * dt; }
        if input.left  { self.x -= self.speed * dt; }
        if input.right { self.x += self.speed * dt; }
    }
}
