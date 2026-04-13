# Subtask 1.1: Define GameState Struct

## Description
Create `game/src/sim/mod.rs` (or `game/src/sim/state.rs`) and define a `GameState` struct that contains every field currently owned by `Game` in `game/src/game.rs`. Derive `Clone`, `serde::Serialize`, `serde::Deserialize`, and `PartialEq` on `GameState` and all of its nested types. This struct becomes the canonical representation of simulation state at any given tick.

## Layer
`game/` crate — `game/src/sim/state.rs`

## Steps
- [ ] Create `game/src/sim/mod.rs` and declare `pub mod state;`.
- [ ] Add `pub mod sim;` to `game/src/lib.rs`.
- [ ] Define `GameState` in `game/src/sim/state.rs` with fields: `player: Player`, `enemies: Vec<Enemy>`, `bullets: Vec<Bullet>`, `impacts: Vec<ImpactMark>`, `walls: Vec<Wall>`, `props: Vec<ResolvedProp>`, `rooms: LevelRooms`, `level_bounds: Option<LevelBounds>`.
- [ ] Add `#[derive(Clone, serde::Serialize, serde::Deserialize, PartialEq)]` to `GameState`.
- [ ] Audit every nested type (`Player`, `Enemy`, `Bullet`, `ImpactMark`, `Wall`, `ResolvedProp`, `LevelRooms`, `LevelBounds`, `Movement`, `Sight`, `AimCone`, `WeaponLoadout`, etc.) and add missing `#[derive(Clone, Serialize, Deserialize, PartialEq)]` to each.
- [ ] For any type that cannot derive `PartialEq` due to floating-point fields, implement `PartialEq` manually using an epsilon comparison and document the tolerance used (`1e-5`).
- [ ] Decide on `SpawnQueue`: exclude it from `GameState` (spawn queue is always flushed before state snapshot). Add a `// INVARIANT: SpawnQueue is always empty at snapshot boundaries` comment in `step()`.
- [ ] Verify that `game` crate compiles with `cargo check -p game`.

## Acceptance Criteria
- `GameState` is in `game/src/sim/state.rs`, publicly re-exported from `game/src/sim/mod.rs`.
- `cargo check -p game` passes with no errors.
- All nested types derive or implement `Clone`, `Serialize`, `Deserialize`, `PartialEq`.
- No compile warnings about missing trait implementations.

## Notes
- `fontdue` and `wgpu` types must never appear in `GameState` — they live in the `engine` crate which is never a dependency of `game`.
- `ResolvedProp` in `game/src/world/prop.rs` may contain texture/asset references — audit and remove or replace with an asset ID (e.g., `prop_asset_id: u32`) for serialization.
- `LevelRooms` (`game/src/world/rooms.rs`) is computed from walls at level load and can be re-derived — consider marking it `#[serde(skip)]` and recomputing on deserialization if it contains non-trivial data structures.

## Dependencies
- None.
