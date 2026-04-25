use ecs::World;

use crate::components::Health;
use crate::events::{DamageEvent, DestroyEvent};

/// Consumes all `DamageEvent`s queued this tick, applies damage to the target
/// entity's `Health`, and emits a `DestroyEvent` for any entity that reaches
/// zero hit points.
///
/// Type-agnostic: applies to any entity that has a `Health` component.
pub fn run(world: &mut World) {
    let events = world.events.drain::<DamageEvent>();
    for ev in events {
        if let Some(health) = world.get_mut::<Health>(ev.target) {
            health.current = health.current.saturating_sub(ev.amount);
            if health.is_dead() {
                world.events.emit(DestroyEvent { entity: ev.target });
            }
        }
    }
}
