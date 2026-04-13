# Subtask 1.2: Test GameState Serialization

## Description
Write unit tests that construct a `GameState`, serialize it to JSON via `serde_json`, deserialize it back, and assert equality. This validates that no data is silently dropped during round-trip and that `PartialEq` on `GameState` behaves correctly.

## Layer
`game/` crate — `game/src/sim/state.rs` (inline `#[cfg(test)]` module) or `game/tests/sim_serialization.rs`.

## Steps
- [ ] Add a `#[cfg(test)]` module to `game/src/sim/state.rs`.
- [ ] Implement a `GameState::default()` (or a test helper `fn sample_state() -> GameState`) that constructs a minimal but non-trivial state (at least one player, one enemy, one bullet, one wall).
- [ ] Write test `serialize_deserialize_round_trip`: serialize `sample_state()` to JSON string, deserialize back, assert `original == deserialized`.
- [ ] Write test `clone_equality`: assert `state == state.clone()`.
- [ ] Write test `json_is_not_empty`: assert serialized JSON length > 50 bytes (guards against accidentally serializing an empty struct).
- [ ] Run `cargo test -p game` and confirm all tests pass.

## Acceptance Criteria
- All three tests pass under `cargo test -p game`.
- No `#[allow(dead_code)]` suppressions needed for tested types.
- Serialized JSON contains `"player"`, `"enemies"`, `"bullets"`, `"walls"` keys.

## Notes
- Use `serde_json::to_string` and `serde_json::from_str` for the round-trip. No custom serializer needed at this stage.
- If `PartialEq` is implemented with epsilon tolerance, document the chosen epsilon value in a comment inside the test.

## Dependencies
- Subtask 1.1 (`1_1_define_game_state_struct`) — `GameState` must exist before it can be tested.
