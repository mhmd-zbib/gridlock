use std::time::Instant;

/// Fixed timestep in seconds (60 Hz).
pub const FIXED_STEP: f32 = 1.0 / 60.0;

/// Drives the fixed-update / render split.
///
/// Call [`GameLoop::tick`] every frame. It will call `update` as many times
/// as needed to consume accumulated time in fixed steps, then call `render` once.
pub struct GameLoop {
    last_time: Instant,
    accumulator: f32,
}

impl GameLoop {
    pub fn new() -> Self {
        Self {
            last_time: Instant::now(),
            accumulator: 0.0,
        }
    }

    pub fn tick(&mut self, mut update: impl FnMut(f32), render: impl FnOnce()) {
        let steps = self.consume_fixed_steps();
        for _ in 0..steps {
            update(FIXED_STEP);
        }
        render();
    }

    /// Advance wall-clock state and return how many fixed updates should run.
    pub fn consume_fixed_steps(&mut self) -> u32 {
        let now = Instant::now();
        let dt = now.duration_since(self.last_time).as_secs_f32();
        self.last_time = now;

        // Cap to avoid the "spiral of death" after a stall / debugger pause.
        self.accumulator += dt.min(0.25);

        let mut steps = 0_u32;
        while self.accumulator >= FIXED_STEP {
            self.accumulator -= FIXED_STEP;
            steps += 1;
        }
        steps
    }
}
