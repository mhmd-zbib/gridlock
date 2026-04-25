use ecs::World;

use crate::components::{Position, VisibilityState};
use crate::events::VisibilityChangedEvent;
use crate::world::sight::Sight;
use crate::world::wall::Wall;

/// Computes visibility scores for every entity with `VisibilityState` from
/// every entity with a `Sight` component.
///
/// Output formula (per observer → target pair):
/// `visibility_score = if sight.can_see(obs_pos, target_pos, walls) { 1.0 } else { 0.0 }`
///
/// Emits `VisibilityChangedEvent` when the score changes.  Type-agnostic:
/// any entity with a `Sight` can observe, any entity with `VisibilityState`
/// can be observed.
pub fn run(world: &mut World, walls: &[Wall]) {
    // Collect observer data (entity + position + sight config).
    let observers: Vec<_> = world
        .entities_with::<Sight>()
        .into_iter()
        .filter_map(|e| {
            let pos = world.get::<Position>(e).copied()?;
            let sight = world.get::<Sight>(e).copied();
            sight.map(|s| (e, pos, s))
        })
        .collect();

    // Collect targets.
    let targets: Vec<_> = world.entities_with::<VisibilityState>();

    for &target in &targets {
        let target_pos = match world.get::<Position>(target).copied() {
            Some(p) => p,
            None => continue,
        };

        let mut best_score = 0.0_f32;
        for &(obs, obs_pos, ref sight) in &observers {
            if obs == target {
                continue;
            }
            if sight.can_see((obs_pos.x, obs_pos.y), (target_pos.x, target_pos.y), walls) {
                best_score = 1.0;
                break;
            }
        }

        let old_score = world.get::<VisibilityState>(target).map(|v| v.score).unwrap_or(0.0);
        if (best_score - old_score).abs() > f32::EPSILON {
            world.events.emit(VisibilityChangedEvent {
                observer: ecs::Entity::new(0),
                target,
                score: best_score,
            });
        }

        if let Some(vs) = world.get_mut::<VisibilityState>(target) {
            vs.score = best_score;
        }
    }
}
