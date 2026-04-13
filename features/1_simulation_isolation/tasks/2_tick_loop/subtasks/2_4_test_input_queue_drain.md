# Subtask 2.4: Test Input Queue Drain

## Description
Write unit tests for `InputQueueHandle` verifying that pushed inputs are returned by `drain`, the queue is empty after a drain, and the 128-input cap is enforced.

## Layer
`server/` crate — `server/src/input_queue.rs` inline `#[cfg(test)]` module.

## Steps
- [ ] Write test `push_and_drain_returns_inputs`:
  - Create a `InputQueueHandle`.
  - Push 3 `PlayerInput` values with sequences 1, 2, 3.
  - Call `drain()` and assert the returned `Vec` has length 3.
  - Assert sequences are 1, 2, 3 in order.
- [ ] Write test `drain_clears_queue`:
  - Push 2 inputs.
  - Call `drain()` once.
  - Call `drain()` again — assert returned `Vec` is empty.
- [ ] Write test `drain_caps_at_128`:
  - Push 200 inputs with sequences 0..200.
  - Call `drain()`.
  - Assert returned `Vec` length is 128.
  - (The 72 oldest inputs should have been discarded.)
- [ ] Write test `concurrent_push_drain`:
  - Spawn a thread that pushes 50 inputs.
  - Main thread sleeps 1 ms then drains.
  - Assert total drained eventually equals 50 after joining.
- [ ] Run `cargo test -p server` and confirm all pass.

## Acceptance Criteria
- All four tests pass under `cargo test -p server`.
- No deadlocks observed.
- Cap behavior: when 200 inputs are pushed, only 128 are returned.

## Notes
- The cap policy is: when the queue exceeds 128, the oldest (front) inputs are dropped and the newest are kept. This prioritizes the most recent inputs.

## Dependencies
- Subtask 2.3 (`2_3_implement_input_queue`) — `InputQueueHandle` must be implemented.
