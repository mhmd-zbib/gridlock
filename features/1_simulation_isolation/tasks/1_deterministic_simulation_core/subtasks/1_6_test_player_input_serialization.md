# Subtask 1.6: Test PlayerInput Serialization

## Description
Write unit tests confirming that `PlayerInput` serializes to JSON and deserializes back without loss, and that two inputs with different `sequence` values are not equal. This ensures the type is safe to use as a network message.

## Layer
`game/` crate — `game/src/sim/input.rs` inline `#[cfg(test)]` module.

## Steps
- [ ] Write test `player_input_round_trip`:
  - Construct a `PlayerInput` with `sequence: 42`, `move_right: true`, `shoot: true`, `aim_x: 1.0`, `aim_y: 0.0`, all other fields false/0.
  - Serialize to JSON string with `serde_json::to_string`.
  - Deserialize back with `serde_json::from_str`.
  - Assert `original == deserialized`.
- [ ] Write test `different_sequences_not_equal`:
  - Construct two identical `PlayerInput` except `sequence` differs (1 vs 2).
  - Assert they are not equal.
- [ ] Write test `default_input_is_all_false`:
  - Implement `Default` for `PlayerInput` (all bools false, floats 0.0, sequence 0).
  - Assert `PlayerInput::default().shoot == false`.
- [ ] Run `cargo test -p game` and confirm all pass.

## Acceptance Criteria
- All three tests pass.
- `PlayerInput` implements `Default` with all-zero/false values.
- JSON output contains `"sequence"`, `"shoot"`, `"reload"` keys.

## Notes
- Add `impl Default for PlayerInput` in `game/src/sim/input.rs`.

## Dependencies
- Subtask 1.5 (`1_5_define_player_input_type`) — `PlayerInput` must be defined.
