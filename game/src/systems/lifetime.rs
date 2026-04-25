use ecs::World;

use crate::components::Lifetime;
use crate::events::DestroyEvent;

/// Ticks every entity's `Lifetime` component and emits `DestroyEvent` for
/// expired entities.  Type-agnostic: works on any entity with `Lifetime`.
pub fn run(world: &mut World, dt: f32) {
    let entities: Vec<_> = world.entities_with::<Lifetime>();
    for entity in entities {
        if let Some(lt) = world.get_mut::<Lifetime>(entity) {
            lt.remaining -= dt;
            if lt.is_expired() {
                world.events.emit(DestroyEvent { entity });
            }
        }
    }
}
