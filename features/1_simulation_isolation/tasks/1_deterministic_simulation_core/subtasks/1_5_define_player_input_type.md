# Subtask 1.5: Define PlayerInput Type

## Description
Define `PlayerInput` in `game/src/sim/input.rs` — a serializable struct representing a single player's actions for one tick. This type is stamped with a monotonically increasing `sequence` number on the client, sent to the server, and held in the client's prediction buffer. It replaces the renderer-coupled `InputState` at the simulation boundary.

## Layer
`game/` crate — `game/src/sim/input.rs`

## Steps
- [ ] Create `game/src/sim/input.rs` and declare it in `game/src/sim/mod.rs`.
- [ ] Define:
  ```rust
  #[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
  pub struct PlayerInput {
      pub sequence: u32,       // monotonically increasing, wraps at u32::MAX
      pub move_up: bool,
      pub move_down: bool,
      pub move_left: bool,
      pub move_right: bool,
      pub aim_x: f32,          // world-space aim direction x (normalized)
      pub aim_y: f32,          // world-space aim direction y (normalized)
      pub shoot: bool,
      pub reload: bool,
  }
  ```
- [ ] Implement `PlayerInput::from_input_state(seq: u32, raw: &InputState, mouse_world_x: f32, mouse_world_y: f32) -> Self` as a conversion helper used by the client layer.
- [ ] Add `pub use sim::input::PlayerInput;` to `game/src/lib.rs` for downstream visibility.
- [ ] Run `cargo check -p game` to confirm no errors.

## Acceptance Criteria
- `PlayerInput` is in `game/src/sim/input.rs`, publicly exported from `game`.
- All fields are `Serialize`, `Deserialize`, `Clone`, `PartialEq`.
- `from_input_state` compiles and converts the existing `InputState` correctly.
- `cargo check -p game` passes.

## Notes
- `sequence: u32` is intentionally not a `u64` — it is transmitted over UDP frequently and size matters. Wrapping after ~4 billion inputs is acceptable.
- `aim_x`/`aim_y` are world-space normalized direction vectors, not raw pixel coordinates. The conversion from screen pixels to world coordinates belongs in the `client` crate, not in `game`.
- Do not remove `InputState` from `game/src/input.rs` yet — `client` depends on it for the renderer input pipeline.

## Dependencies
- Subtask 1.1 (`1_1_define_game_state_struct`) — `game/src/sim/mod.rs` must exist.
