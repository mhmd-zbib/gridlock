# Subtask 2.3: Implement Input Queue

## Description
Define `InputQueueHandle` — a thread-safe handle to a per-player input queue that the network receive path pushes into and the tick loop drains from. The tick loop calls `drain()` at the start of each tick to collect all pending inputs for that player, then clears the queue.

## Layer
`server/` crate — `server/src/input_queue.rs`

## Steps
- [ ] Create `server/src/input_queue.rs`.
- [ ] Define:
  ```rust
  use std::collections::VecDeque;
  use std::sync::{Arc, Mutex};
  use game::sim::input::PlayerInput;

  #[derive(Clone)]
  pub struct InputQueueHandle {
      inner: Arc<Mutex<VecDeque<PlayerInput>>>,
  }

  impl InputQueueHandle {
      pub fn new() -> Self { ... }
      /// Called by the network receive path to enqueue a received input.
      pub fn push(&self, input: PlayerInput) { ... }
      /// Called by the tick loop to drain all pending inputs for this tick.
      pub fn drain(&self) -> Vec<PlayerInput> { ... }
  }
  ```
- [ ] `push` acquires the lock, pushes to back of `VecDeque`.
- [ ] `drain` acquires the lock, drains the `VecDeque` into a `Vec`, returns the `Vec`. Limit max drain to 128 inputs per tick (discard older inputs if queue exceeds 128 to prevent runaway memory growth from a misbehaving client).
- [ ] Add `pub mod input_queue;` to `server/src/main.rs` or `server/src/lib.rs`.
- [ ] Run `cargo check -p server`.

## Acceptance Criteria
- `InputQueueHandle` is `Clone` and `Send + Sync`.
- `push` and `drain` operate correctly under concurrent access.
- Queue is bounded at 128 inputs (older inputs dropped when over limit).
- `cargo check -p server` passes.

## Notes
- The 128-input cap prevents a slow-network client from causing the server to process thousands of stale inputs in a single tick on reconnect.
- `Arc<Mutex<VecDeque>>` is appropriate here — this is a low-contention path (one writer, one reader per queue).

## Dependencies
- Feature 1 Task 1 subtask 1.5 (`1_5_define_player_input_type`) — `PlayerInput` must exist.
