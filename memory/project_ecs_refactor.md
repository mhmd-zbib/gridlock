---
name: ECS Architecture Refactor
description: Full ECS refactor completed — new ecs crate, World-backed Game, type-agnostic systems, event bus
type: project
---

Completed a full ECS architecture refactor of the Rust shooting game.

**Why:** Task requested strict separation between engine runtime and game logic, with entities as IDs, components as data, and type-agnostic systems.

**What changed:**
- New `ecs/` crate: `World`, `Entity(u64)`, `Component`/`Event`/`Resource` trait markers, `EventBus`, `System` trait
- `game/src/components.rs`: `Position`, `Velocity`, `Speed`, `VelocityFrac`, `CollisionRadius`, `Health`, `Damage`, `Lifetime`, `VisibilityState`, `BulletData`, `PlayerController`, `AiAgent`, `PlayerTag`, `BotTag`, `BulletTag`, `ImpactTag`
- `game/src/events.rs`: `SpawnBulletEvent`, `SpawnEnemyEvent`, `DestroyEvent`, `DamageEvent`, `BulletTraceEvent`, `SoundEvent`, `VisibilityChangedEvent`
- `game/src/bundles.rs`: `spawn_player`, `spawn_enemy`, `spawn_bullet`, `spawn_impact`
- `game/src/systems/`: `input`, `ai`, `movement`, `physics`, `projectile`, `visibility`, `sound`, `lifetime`, `combat`, `spawn`
- `game/src/game.rs`: `Game` struct backed by `World`; provides `EnemyView`, `ImpactView`, accessor methods
- Entity structs (`Player`, `Enemy`) removed; `EnemyKind` kept; `PlayerLoadoutConfig` kept
- Client updated to use `game.player_pos()`, `game.player_sight()`, `game.enemy_views()`, etc.

**How to apply:** The `ecs` crate is a workspace member. `game` depends on `ecs`. Systems are free functions called in order by `Game::update()`. No entity-specific logic in systems — all operate on component queries.
