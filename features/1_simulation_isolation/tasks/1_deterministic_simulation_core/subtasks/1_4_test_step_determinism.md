# Subtask 1.4: Test Step Determinism

## Description
Write a test that verifies `step()` is deterministic: calling it twice from the same initial `GameState` with identical `PlayerInput` values and identical `dt` produces equal output states. This is the core correctness guarantee that prediction and reconciliation depend on.

## Layer
`game/` crate — `game/src/sim/step.rs` inline `#[cfg(test)]` module.

## Steps
- [ ] Write test `step_is_deterministic`:
  - Construct a `GameState` with a player at a known position, one enemy, one wall, no bullets.
  - Clone the state into `state_a` and `state_b`.
  - Construct a `PlayerInput` with `move_right: true`, `shoot: false`, `sequence: 1`.
  - Call `step(&mut state_a, &[input.clone()], 1.0 / 60.0)`.
  - Call `step(&mut state_b, &[input.clone()], 1.0 / 60.0)`.
  - Assert `state_a == state_b`.
- [ ] Write test `step_advances_player_position`:
  - Construct a state with player at `(0.0, 0.0)`.
  - Apply an input with `move_right: true`.
  - Assert `state.player.movement.x > 0.0` after one step.
- [ ] Run `cargo test -p game` and confirm all tests pass.

## Acceptance Criteria
- `step_is_deterministic` passes — both states are equal after identical calls.
- `step_advances_player_position` passes — movement is non-zero.
- No test relies on wall-clock time or `rand` seeding.

## Notes
- If `PartialEq` uses epsilon comparison, a single-step difference of less than `1e-5` in `f32` values is acceptable.
- If the test fails due to non-determinism, the most likely cause is a side effect inside `step` (e.g., `Instant::now()`, `thread_rng()`). Grep for these in the call tree.

## Dependencies
- Subtask 1.3 (`1_3_extract_step_function`) — `step` must exist before it can be tested.
