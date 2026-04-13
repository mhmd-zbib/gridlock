# Task 1: Deterministic Simulation Core

## Description
Define a serializable `GameState` struct that captures every field necessary to reconstruct the simulation at a given tick, and refactor `Game::update` into a free function `step(state: &mut GameState, inputs: &[PlayerInput], dt: f32)` that has no side effects beyond mutating the provided state. This function must be callable identically on server and client — it is the prerequisite for client-side prediction, reconciliation, and server authority.

## Layer
`game/` crate — `game/src/sim/` (new module). This crate is the shared simulation layer. It has no renderer, no network, no OS I/O.

## Dependencies
- None. This is task 1 of feature 1.

## Acceptance Criteria
- `GameState` implements `Clone`, `serde::Serialize`, `serde::Deserialize`, and `PartialEq`.
- `step()` is a free function (not a method): signature `fn step(state: &mut GameState, inputs: &[PlayerInput], dt: f32)`.
- Calling `step()` twice with identical arguments from identical starting states produces byte-identical `GameState` outputs.
- Existing `Game` struct in `game/src/game.rs` is refactored to delegate to `step()` — no logic duplication.
- Zero use of global mutable state, `thread_local!`, `static mut`, or system time inside `step()`.
- All existing unit tests in `game/` continue to pass.

## Notes
- The existing `Game::update(&mut self, dt, input)` is already close to a pure function. The main work is: (1) extracting all mutable fields into a `GameState` struct, (2) making `step` a free function, and (3) deriving `Serialize`/`Deserialize`/`Clone`/`PartialEq` on all sub-structs.
- `ImpactMark`, `Bullet`, `Enemy`, `Player`, `Wall`, `ResolvedProp`, `LevelRooms`, `LevelBounds` all need `Serialize`/`Deserialize`. Some already derive `serde` traits — audit each.
- Floating-point determinism: `f32` arithmetic on the same CPU architecture is deterministic per IEEE 754. Document explicitly that server and client must run the same Rust build target. Cross-platform determinism is not guaranteed and is a known limitation.
- Do not remove `Game` struct yet — wrap it or keep it as a compatibility shim until the rest of the codebase migrates.
- `SpawnQueue` must be captured in `GameState` or flushed before state serialization — decide which and document the choice.

## Subtasks
1. `1_1_define_game_state_struct` – Define `GameState`, derive serialization and equality, migrate all sim fields into it.
2. `1_2_test_game_state_serialization` – Verify `GameState` round-trips through `serde_json` without data loss.
3. `1_3_extract_step_function` – Refactor `Game::update` body into `pub fn step(state: &mut GameState, inputs: &[PlayerInput], dt: f32)`.
4. `1_4_test_step_determinism` – Write a test that calls `step` twice from the same initial state with identical inputs and asserts output equality.
5. `1_5_define_player_input_type` – Define `PlayerInput` struct with all action fields plus a `sequence: u32` field for prediction.
6. `1_6_test_player_input_serialization` – Verify `PlayerInput` serializes and deserializes correctly.
