use crate::entity::movement::{Movement, MovementInput};
use crate::systems::movement::{PEEK_DISTANCE, PEEK_LERP_SPEED, clamped_peek_distance};
use crate::world::wall::Wall;
use crate::world::units::px_to_tiles;
use engine::input::InputState;

const WALK_SPEED: f32 = px_to_tiles(40.0);
const NORMAL_SPEED: f32 = px_to_tiles(85.0);
pub const RUN_SPEED: f32 = px_to_tiles(200.0);
const SPRINT_MAX_SECS: f32 = 1.2;
const SPRINT_COOLDOWN_RATIO: f32 = 3.0;

pub struct LocomotionState {
    peek_origin: Option<(f32, f32)>,
    sprint_left: f32,
    sprint_cooldown: f32,
}

impl LocomotionState {
    pub fn new() -> Self {
        Self {
            peek_origin: None,
            sprint_left: SPRINT_MAX_SECS,
            sprint_cooldown: 0.0,
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
        self.tick_sprint(dt, input, movement);
        let dir = input_dir(input);
        apply_peek_or_walk(dt, input, movement, dir, half_size, walls, &mut self.peek_origin);
    }

    fn tick_sprint(&mut self, dt: f32, input: &InputState, movement: &mut Movement) {
        if self.sprint_cooldown > 0.0 {
            let recovered = dt.min(self.sprint_cooldown) / SPRINT_COOLDOWN_RATIO;
            self.sprint_left = (self.sprint_left + recovered).min(SPRINT_MAX_SECS);
            self.sprint_cooldown = (self.sprint_cooldown - dt).max(0.0);
        }

        let sprinting = input.shift && self.sprint_cooldown <= 0.0 && self.sprint_left > 0.0;
        if sprinting {
            let used = dt.min(self.sprint_left);
            self.sprint_left -= used;
            if self.sprint_left <= 0.0 {
                self.sprint_cooldown = SPRINT_MAX_SECS * SPRINT_COOLDOWN_RATIO;
            }
        }

        movement.speed = if sprinting {
            RUN_SPEED
        } else if input.walk {
            WALK_SPEED
        } else {
            NORMAL_SPEED
        };
    }
}

// ---------------------------------------------------------------------------
// Peek movement
// ---------------------------------------------------------------------------

fn input_dir(input: &InputState) -> (f32, f32) {
    let px = if input.right { 1.0 } else if input.left { -1.0 } else { 0.0 };
    let py = if input.down { 1.0 } else if input.up { -1.0 } else { 0.0 };
    (px, py)
}

fn apply_peek_or_walk(
    dt: f32,
    input: &InputState,
    movement: &mut Movement,
    dir: (f32, f32),
    half_size: f32,
    walls: &[Wall],
    peek_origin: &mut Option<(f32, f32)>,
) {
    let has_dir = dir.0 != 0.0 || dir.1 != 0.0;

    if input.peek && peek_origin.is_none() {
        *peek_origin = Some((movement.x, movement.y));
    }

    let Some(origin) = *peek_origin else {
        movement.apply(MovementInput::from_input(input), dt);
        return;
    };

    // Peek key released while moving → exit peek immediately and walk.
    if !input.peek && has_dir {
        *peek_origin = None;
        movement.apply(MovementInput::from_input(input), dt);
        return;
    }

    apply_peek(dt, input, movement, dir, has_dir, half_size, walls, peek_origin, origin);
}

fn apply_peek(
    dt: f32,
    input: &InputState,
    movement: &mut Movement,
    dir: (f32, f32),
    has_dir: bool,
    half_size: f32,
    walls: &[Wall],
    peek_origin: &mut Option<(f32, f32)>,
    origin: (f32, f32),
) {
    let target = if input.peek && has_dir {
        let len = (dir.0 * dir.0 + dir.1 * dir.1).sqrt();
        let norm = (dir.0 / len, dir.1 / len);
        let dist = clamped_peek_distance(origin, norm, PEEK_DISTANCE, half_size, walls);
        (origin.0 + norm.0 * dist, origin.1 + norm.1 * dist)
    } else {
        origin
    };

    let alpha = (PEEK_LERP_SPEED * dt).min(1.0);
    movement.x += (target.0 - movement.x) * alpha;
    movement.y += (target.1 - movement.y) * alpha;
    movement.velocity_frac = 0.0;

    // Once fully returned to origin, exit peek mode.
    if !input.peek {
        let dx = movement.x - origin.0;
        let dy = movement.y - origin.1;
        if dx * dx + dy * dy < 1.0e-6 {
            movement.x = origin.0;
            movement.y = origin.1;
            *peek_origin = None;
        }
    }
}

