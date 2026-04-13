# Subtask 2.2: Test Tick Loop Timing

## Description
Write a test that spawns the tick loop on a background thread, lets it run for approximately 200 ms, then counts how many `TickOutput` messages were received on the channel and asserts the count is within ±3 of the expected 12 ticks (200 ms at 60 Hz = 12 ticks).

## Layer
`server/` crate — `server/src/tick_loop.rs` inline `#[cfg(test)]` module.

## Steps
- [ ] Add a `#[cfg(test)]` module to `server/src/tick_loop.rs`.
- [ ] Write test `tick_loop_fires_at_correct_rate` (mark `#[ignore]` to avoid running in CI by default since it is timing-dependent):
  - Construct a minimal `GameState` (use `GameState::default()` or a helper).
  - Create `std::sync::mpsc::channel::<TickOutput>()`.
  - Spawn a thread running `run_tick_loop(state, /* no-op input handle */, tx)`.
  - Sleep the test thread for 200 ms.
  - Drain `rx` with `try_recv` in a loop, counting messages.
  - Assert `count >= 9 && count <= 15` (wide tolerance for CI).
- [ ] Run `cargo test -p server -- tick_loop_fires_at_correct_rate --ignored` and confirm it passes on the development machine.

## Acceptance Criteria
- Test compiles and the tick count falls in `[9, 15]` on the development machine.
- The test is marked `#[ignore]` so it does not block regular `cargo test` runs.

## Notes
- Timing tests are inherently imprecise in CI. The `#[ignore]` flag is intentional.
- If the count is consistently 0, the likely issue is that the channel is full and `send` is blocking. Use `try_send` or ensure the receiver drains fast enough.

## Dependencies
- Subtask 2.1 (`2_1_implement_tick_loop`) — the loop must be implemented.
