# Task 2: Server Tick Loop

## Description
Implement the authoritative headless tick loop inside `server/src/` that runs at a fixed tick rate (60 Hz for this shooter), drains a per-player input queue each tick, calls `game::sim::step()`, and produces an output channel of `(tick: u64, GameState)` pairs for the snapshot broadcasting system to consume. The loop must be time-accurate: if processing takes longer than the tick budget, it catches up (bounded) rather than falling behind indefinitely.

## Layer
`server/` crate — `server/src/tick_loop.rs` and `server/src/main.rs`.

## Dependencies
- Task 1 (`1_deterministic_simulation_core`) — `GameState`, `PlayerInput`, and `step()` must all exist before the tick loop can call them.

## Acceptance Criteria
- The loop runs at exactly 60 Hz (16.667 ms per tick) as measured by wall-clock intervals between tick completions.
- If one tick takes longer than the budget, the next tick fires immediately (catch-up), but catch-up is capped at 5 ticks maximum to prevent spiral-of-death.
- Each tick publishes `(tick_number: u64, GameState)` to a `std::sync::mpsc` channel (or `tokio::sync::broadcast` if async).
- The tick loop logs a warning to stderr when a tick exceeds its budget.
- `cargo build -p server` succeeds.

## Notes
- Use `std::thread::sleep` with `Duration` arithmetic for the timing loop initially; switch to `tokio::time::interval` if async runtime is added in Feature 2.
- The input queue is a `VecDeque<PlayerInput>` per player slot, protected by a `Mutex`. The tick loop drains it at the start of each tick.
- Tick rate constant: `pub const TICK_RATE_HZ: u32 = 60;` in `server/src/config.rs`.
- "Headless" means no window, no renderer, no wgpu. `server/Cargo.toml` must never depend on `engine`.

## Subtasks
1. `2_1_implement_tick_loop` – Implement the 60 Hz fixed-rate loop with catch-up logic and output channel.
2. `2_2_test_tick_loop_timing` – Write a test that runs the loop for 100 ms and asserts approximately 6 ticks fired.
3. `2_3_implement_input_queue` – Implement the per-player `Mutex<VecDeque<PlayerInput>>` input queue and its drain-and-clear operation at tick start.
4. `2_4_test_input_queue_drain` – Write a test that enqueues inputs, fires one tick, and asserts the queue is empty and inputs were applied.
