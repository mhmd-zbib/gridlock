# Subtask 2.1: Implement Tick Loop

## Description
Implement `pub fn run_tick_loop(initial_state: GameState, input_rx: InputQueueHandle, tick_tx: Sender<TickOutput>)` in `server/src/tick_loop.rs`. This function blocks the calling thread and runs at 60 Hz, calling `game::sim::step()` each tick and sending `TickOutput { tick: u64, state: GameState }` on `tick_tx`.

## Layer
`server/` crate — `server/src/tick_loop.rs`

## Steps
- [ ] Add `game = { workspace = true }` to `server/Cargo.toml`.
- [ ] Create `server/src/config.rs` with `pub const TICK_RATE_HZ: u32 = 60;` and `pub const TICK_DURATION: Duration = Duration::from_nanos(1_000_000_000 / TICK_RATE_HZ as u64);`.
- [ ] Define in `server/src/tick_loop.rs`:
  ```rust
  pub struct TickOutput {
      pub tick: u64,
      pub state: GameState,
  }
  ```
- [ ] Implement the tick loop:
  ```
  let mut next_tick_at = Instant::now();
  let mut tick_number: u64 = 0;
  let mut catch_up = 0u32;
  const MAX_CATCH_UP: u32 = 5;
  loop {
      let now = Instant::now();
      if now < next_tick_at {
          std::thread::sleep(next_tick_at - now);
      } else {
          catch_up += 1;
          if catch_up > MAX_CATCH_UP { catch_up = MAX_CATCH_UP; }
      }
      let tick_start = Instant::now();
      // drain inputs
      // call step()
      // send TickOutput
      tick_number += 1;
      next_tick_at += TICK_DURATION;
      let elapsed = tick_start.elapsed();
      if elapsed > TICK_DURATION {
          eprintln!("[tick] tick {} over budget: {:?}", tick_number, elapsed);
      }
  }
  ```
- [ ] Call `game::sim::step(&mut state, &inputs, TICK_DURATION.as_secs_f32())` inside the loop.
- [ ] Send `TickOutput { tick: tick_number, state: state.clone() }` on `tick_tx` after each step. Use `let _ = tick_tx.send(...)` to ignore send errors (receiver may be slow).
- [ ] Wire up `run_tick_loop` in `server/src/main.rs` with a dummy initial state and a channel.
- [ ] Run `cargo build -p server` and confirm success.

## Acceptance Criteria
- `server/src/tick_loop.rs` compiles without errors.
- The loop advances `tick_number` every call.
- `TickOutput` is sent on `tick_tx` each tick.
- `cargo build -p server` succeeds.

## Notes
- Use `std::sync::mpsc::Sender` for `tick_tx` initially — no async runtime required yet.
- The catch-up cap of 5 ticks prevents the simulation from running at 10x speed after a stall.
- `state.clone()` is O(n) in entity count — profile if it becomes a bottleneck. A double-buffer pattern can avoid the clone later.

## Dependencies
- Feature 1 Task 1 subtasks 1.1, 1.3, 1.5 must be complete (`GameState`, `step`, `PlayerInput` all defined).
