# Feature 1: Simulation Isolation

## Overview
The simulation layer is the foundation all multiplayer features depend on. It must be a pure function: given a game state and a set of inputs, it produces a new game state without side effects, global state, or renderer coupling. This isolation enables the server to run headlessly, the client to run prediction locally, and both to produce identical results from the same inputs. Without this property, client-side prediction and lag compensation cannot be implemented correctly.

## Tools & Context
- **Rust workspace (`game` crate)**: Houses all shared simulation logic (`game/src/`). This crate is already consumed by both `client` and `server`.
- **`serde`**: Serialization of `GameState` snapshots for network transport — already in `game/Cargo.toml`.
- **Deterministic arithmetic**: All physics uses `f32` with fixed-step integration; floating-point non-determinism is a known risk and must be documented.
- **`InputState` (`game/src/input.rs`)**: Already exists — must be extended with a sequence number for prediction.

## Layer Placement
- `game/` crate: All simulation logic lives here. This crate has no renderer, no OS I/O, no network. It is the shared simulation layer consumed by both `server/` and `client/`.
- `server/` crate: Owns the authoritative tick loop that drives the simulation.
- `client/` crate: Consumes the simulation for local prediction only.

## Tasks
1. [ ] Deterministic Simulation Core – Extract `Game::update` into a pure `step(state, inputs, dt) -> GameState` function and define a serializable `GameState` type.
2. [ ] Server Tick Loop – Implement the fixed-rate headless tick loop in `server/` that drains an input queue, advances simulation, and produces per-tick output.

## Dependencies
- None. This is the foundational feature. All other features depend on it.
