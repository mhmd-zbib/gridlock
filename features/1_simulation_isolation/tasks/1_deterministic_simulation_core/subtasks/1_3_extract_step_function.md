# Subtask 1.3: Extract Step Function

## Description
Refactor the body of `Game::update` in `game/src/game.rs` into a free function `pub fn step(state: &mut GameState, inputs: &[PlayerInput], dt: f32)` located in `game/src/sim/step.rs`. The function must accept only `GameState`, player inputs, and delta time — no renderer references, no OS calls, no network I/O, no random number generator seeded from system time.

## Layer
`game/` crate — `game/src/sim/step.rs`

## Steps
- [ ] Create `game/src/sim/step.rs` and declare it in `game/src/sim/mod.rs`.
- [ ] Move the logic from `Game::update` into `pub fn step(state: &mut GameState, inputs: &[PlayerInput], dt: f32)`. Map fields: `state.player` ↔ `self.player`, `state.enemies` ↔ `self.enemies`, etc.
- [ ] Inside `step`, for each `PlayerInput` in `inputs`, apply movement, facing, weapon tick, and shooting logic to `state.player`. Multi-player support: `inputs` is a slice indexed by player slot.
- [ ] Call `spawn::flush_spawn_queue` at the end of `step` (before returning) to maintain the invariant that `SpawnQueue` is empty at snapshot boundaries. The spawn queue itself is a local variable inside `step`, not part of `GameState`.
- [ ] Retain all existing sub-system calls: `wall::resolve_all`, `clamp_actor_to_level_bounds`, `visibility::sync_enemy_visibility`, `projectile::step_projectiles`, enemy updates, `impacts.retain`.
- [ ] In `game/src/game.rs`, replace the `Game::update` body with a delegation: construct a `PlayerInput` from the `InputState` argument, call `step(&mut self.state, &[input], dt)`.
- [ ] Confirm `cargo build -p game` and `cargo build -p client` both succeed.

## Acceptance Criteria
- `game/src/sim/step.rs` exports `pub fn step(state: &mut GameState, inputs: &[PlayerInput], dt: f32)`.
- `step` contains no calls to `std::time`, `rand::thread_rng`, any renderer type, or any I/O function.
- `Game::update` in `game/src/game.rs` delegates to `step` — no duplicated physics logic.
- `cargo build --workspace` succeeds with no errors.

## Notes
- The current `Game::update` already takes `&InputState` for a single local player. The new signature uses `&[PlayerInput]` to support multiple players — for now, pass a single-element slice from `Game::update`.
- Enemy AI (`enemy.update`) is driven purely by `GameState` fields — no external inputs needed for enemies in single-player. In multiplayer, enemy AI runs only on server.
- Avoid `println!` inside `step` — remove or gate all debug prints behind `cfg(debug_assertions)`.

## Dependencies
- Subtask 1.1 (`1_1_define_game_state_struct`) — `GameState` must be defined.
- Subtask 1.5 (`1_5_define_player_input_type`) — `PlayerInput` must be defined.
