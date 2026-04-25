use ecs::World;

use crate::components::{CollisionRadius, Position};
use crate::world::bounds::clamp_actor_to_level_bounds;
use crate::world::level::LevelBounds;
use crate::world::wall::{self, Wall};

/// Resolves wall collisions and level-bounds clamping for every entity that
/// has both a `Position` and a `CollisionRadius`.  Operates on any entity
/// type — player, bot, or any future moveable entity.
pub fn run(world: &mut World, walls: &[Wall], level_bounds: Option<LevelBounds>) {
    let entities = world.entities_with::<CollisionRadius>();
    for entity in entities {
        let radius = world.get::<CollisionRadius>(entity).copied();
        if let (Some(CollisionRadius(r)), Some(pos)) = (radius, world.get_mut::<Position>(entity))
        {
            wall::resolve_all(&mut pos.x, &mut pos.y, r, walls);
            clamp_actor_to_level_bounds(&mut pos.x, &mut pos.y, r, level_bounds);
        }
    }
}
