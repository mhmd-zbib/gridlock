# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

---

## Commands

```bash
cargo build -p client
cargo build -p server
cargo run -p server
cargo run -p client
cargo test -p game
cargo test -p game -- systems::movement    # single test
cargo clippy --workspace -- -D warnings
cargo fmt --all
./scripts/local-multiplayer.sh 2
```

---

## Rules

### Functions

- One function does one thing at one abstraction level. If you need "and then" inside a function body, split it.
- If a function needs a comment to describe a step, that step is a function.
- Prefer early return over nested logic. Validate preconditions at the top, then do the work.
- Keep functions pure when possible. Push side effects (I/O, network, randomness) to the call site.
- Name functions after what they return or produce, not how they do it.
- A function longer than ~30 lines is a signal to reconsider, not a hard limit — but take the signal seriously.

### Files and Modules

- One file = one bounded concept. If you scroll more than a screen to find something, the file is probably doing too much.
- A file should be describable in one sentence. If it can't, split it.
- Do not mix unrelated concerns in a single file (e.g. entity definition + rendering + serialization).
- Avoid `utils.rs`, `helpers.rs`, `common.rs`. Name the concept: `codec`, `spawn`, `spatial`, `visibility`.
- A module is a boundary, not a container. It should expose a minimal, deliberate surface area.

### Visibility

- Default to private. Use `pub(crate)` for things needed across the workspace internally. Use `pub` only for true external API surface.
- Do not expose fields on structs unless they must be read from outside the module. Use accessor methods or keep them private.
- If you find yourself writing `pub` everywhere, reconsider the module boundary.

### Traits

- Use a trait to express a substitution point or a capability boundary. Not to group loosely related functions.
- If only one concrete type will ever implement a trait, drop the trait and use an `impl` block.
- Prefer generics over `dyn Trait` on hot paths (game systems, rendering loops). Use `dyn Trait` only for true runtime dispatch.
- Keep traits narrow. A trait with more than ~5 methods is likely two traits.
- Do not implement standard traits (`Display`, `From`, `Iterator`) unless the semantics are a natural fit, not just convenient.

### Types

- Encode invariants in types, not in runtime assertions. A type that cannot represent an invalid state is better than a type that checks validity at use.
- Prefer newtypes over raw primitives for domain values: `PlayerId(u32)` over bare `u32`, `Angle(f32)` over bare `f32`.
- Use `enum` for anything with distinct states or variants. Boolean fields that travel together are almost always an enum.
- Use `Option` instead of sentinel values (`-1`, `0`, `u32::MAX`, `""`).
- Use `Result` for operations that can fail. Do not return a bool and silently swallow the error.
- If a struct has more than ~5 fields, ask whether some of them form a sub-concept that deserves its own type.

### Ownership and Borrowing

- Prefer borrowing (`&T`, `&mut T`) over cloning. A clone on a hot path is a correctness smell as much as a perf one.
- Use `Cow<'_, T>` when you sometimes own and sometimes borrow, not clone-always.
- Avoid interior mutability (`RefCell`, `Mutex`) inside game logic. Prefer clear `&mut` ownership through function parameters.
- `Arc<Mutex<T>>` belongs at the async/sync boundary (e.g. inbound packet queue). It should not spread into game logic.
- If you need multiple `&mut` borrows simultaneously, restructure the data or split the struct.

### Error Handling

- Use `?` in library code (`game`, `net`, `engine`). Propagate errors; do not swallow them.
- Use typed errors, not `Box<dyn Error>`, in code that callers need to match on.
- No `unwrap()` outside of tests or genuinely provably-infallible cases. If it truly can't fail, add a comment saying why.
- No `panic!` for expected failure modes: bad input, missing asset, network error. These are `Result::Err`.
- Binary entry points (`main`) may use `expect("meaningful message")` for setup-time failures that are unrecoverable.

### Naming

- Name types after what they are, not what they contain: `Session` not `SessionData`, `Snapshot` not `SnapshotPayload`.
- Name functions after what they produce or do: `build_snapshot`, `apply_input`, `resolve_walls`.
- Use the Rust convention: `new` constructs, `build` finalizes a builder, `from_*` converts, `into_*` consumes and converts.
- Avoid noise words: `Manager`, `Handler`, `Processor`, `Helper`, `Info`, `Wrapper`. Name the domain concept.
- Boolean variables and fields: prefix with `is_`, `has_`, `can_`, `should_` only when it genuinely aids clarity.

### Structs and `impl` Blocks

- Keep `impl` blocks in the same file as the type. Do not scatter `impl Foo` across multiple files.
- Group `impl` blocks: data construction first, then domain methods, then trait implementations last.
- Constructors beyond `new` should be named with intent: `with_team`, `from_loadout`, not `new2` or `create`.
- Derived traits (`Debug`, `Clone`, `PartialEq`) are fine when they make sense. Do not derive `Clone` just to avoid fixing a borrow issue.

### Enums

- Each variant should carry only the data it needs. Do not put a large struct in every variant if only one variant uses it.
- Prefer exhaustive `match`. Avoid `_ =>` arms unless the remaining variants are truly irrelevant and you add a comment.
- Use `#[non_exhaustive]` on enums that will grow over time and are part of a public API.

### Lifetimes

- If you reach for a lifetime annotation, first ask whether ownership transfer (`T` instead of `&T`) would be simpler.
- Named lifetimes in structs are acceptable when the struct is a view into data it does not own. Avoid them in types that should own their data.
- Do not fight the borrow checker with workarounds. It is usually pointing at a design problem.

### Async

- `server` is fully async (tokio). `client` is sync (winit); async runs on a background thread for I/O only. Do not pull tokio into `game`, `engine`, or `net` logic.
- Do not `.await` inside game logic. Game systems are synchronous and must remain so.
- Use `tokio::sync::mpsc` or a shared queue to cross the async/sync boundary. The existing inbound packet queue is the model.
- Prefer `select!` over spawning a task for every small concurrent thing.

### Performance-sensitive Code (tick loop, rendering)

- No allocations inside the 60 Hz tick loop. Pre-allocate buffers and reuse them.
- No `clone()` inside the tick loop or render path.
- Avoid `HashMap` lookups with string keys in hot code. Use typed IDs.
- Spatial queries should go through the spatial index, not iterate all entities.
- Profile before optimizing. A clean algorithm beats a micro-optimized mess.

### SOLID Applied

- **SRP**: Each system file owns one phase of the tick. Adding a mechanic = new file, not extending an existing system.
- **OCP**: New weapon classes, AI behaviors, or attachment effects = new types + trait impls. Not new `match` arms spread across existing code.
- **LSP**: Implement every method of a trait correctly. Do not no-op a required method to satisfy the compiler.
- **ISP**: Narrow traits. A rendering trait for HUD should not include world-rendering methods.
- **DIP**: Systems depend on `Game` (data). They do not call each other. The tick loop is the composer.

### What to Avoid

- Do not add abstraction for its own sake. Three similar lines are better than a premature generalization.
- Do not add error handling for things that structurally cannot happen.
- Do not leave dead code. Remove it or explain with a comment why it will be used.
- Do not mix levels of abstraction in one function (raw pointer math next to business logic).
- Do not use feature flags or compatibility shims when you can just change the code.
- Do not reach for a macro when a function will do.
