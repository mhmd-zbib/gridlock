use ecs::World;

use crate::components::{Position, Speed, VelocityFrac};
use crate::events::SoundEvent;
use crate::world::sound::SoundField;

/// Advances the sound field for every entity that emits sound, then emits a
/// `SoundEvent` for each active emitter so AI systems can react.
///
/// Type-agnostic: any entity with `SoundField + Position + Speed + VelocityFrac`
/// is treated as a sound emitter.
pub fn run(world: &mut World, dt: f32) {
    let emitters: Vec<_> = world.entities_with::<SoundField>();
    for entity in emitters {
        let speed_val = world.get::<Speed>(entity).copied().unwrap_or_default().0;
        let vel_frac = world.get::<VelocityFrac>(entity).copied().unwrap_or_default().0;
        let pos = world.get::<Position>(entity).copied().unwrap_or_default();

        if let Some(sf) = world.get_mut::<SoundField>(entity) {
            sf.tick(speed_val * vel_frac, dt);
            let radius = sf.smoothed_radius;
            world.events.emit(SoundEvent {
                source: entity,
                radius,
                position: (pos.x, pos.y),
            });
        }
    }
}
