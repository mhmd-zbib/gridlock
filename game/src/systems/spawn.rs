use ecs::World;

use crate::bundles;
use crate::entity::enemy::EnemyKind;
use crate::events::{DestroyEvent, SpawnBulletEvent, SpawnEnemyEvent};
use crate::world::ray::cast_ray;
use crate::world::wall::Wall;
use crate::game::BULLET_MAX_RANGE;
use crate::events::BulletTraceEvent;
use crate::entity::bullet::BulletOwner;

/// Materialises all queued `SpawnBulletEvent` and `SpawnEnemyEvent`s into
/// live entities.
///
/// For player-owned bullets, the full trajectory is ray-cast to the first
/// wall and a `BulletTraceEvent` is emitted so the server can broadcast the
/// shot in the same tick it was fired, without waiting for the projectile to
/// travel physically to its destination.
pub fn run(world: &mut World, walls: &[Wall]) {
    let bullet_events = world.events.drain::<SpawnBulletEvent>();
    for ev in bullet_events {
        if ev.owner == BulletOwner::Player {
            let dist = cast_ray((ev.x, ev.y), (ev.dir_x, ev.dir_y), BULLET_MAX_RANGE, walls);
            world.events.emit(BulletTraceEvent {
                x: ev.x + ev.dir_x * dist,
                y: ev.y + ev.dir_y * dist,
                origin_x: ev.x,
                origin_y: ev.y,
            });
        }
        bundles::spawn_bullet(world, ev.x, ev.y, ev.dir_x, ev.dir_y, ev.speed, ev.damage, ev.owner);
    }

    let enemy_events = world.events.drain::<SpawnEnemyEvent>();
    for ev in enemy_events {
        bundles::spawn_enemy(world, ev.x, ev.y, EnemyKind::Shooter);
    }
}

/// Removes all entities for which a `DestroyEvent` was emitted this tick.
pub fn despawn(world: &mut World) {
    let events = world.events.drain::<DestroyEvent>();
    for ev in events {
        if world.is_alive(ev.entity) {
            world.despawn(ev.entity);
        }
    }
}
