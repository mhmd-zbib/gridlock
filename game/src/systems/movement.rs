use ecs::World;

use crate::components::{BulletTag, Position, Velocity};
use crate::world::bounds::clamp_actor_to_level_bounds;
use crate::world::level::LevelBounds;
use crate::world::units::px_to_tiles;
use crate::world::wall::{self, Wall};

pub const PEEK_DISTANCE: f32 = px_to_tiles(18.0);
pub const PEEK_LERP_SPEED: f32 = 12.0;

// ---------------------------------------------------------------------------
// Generic movement system
// ---------------------------------------------------------------------------

/// Applies velocity to position for all bullet entities.
///
/// Actor entities (player and enemies) update their own position through the
/// input and AI systems, which handle the more complex locomotion logic.
pub fn run(world: &mut World, dt: f32) {
    let bullets: Vec<_> = world.entities_with::<BulletTag>();
    for entity in bullets {
        let vel = world.get::<Velocity>(entity).copied();
        if let (Some(vel), Some(pos)) = (vel, world.get_mut::<Position>(entity)) {
            pos.x += vel.x * dt;
            pos.y += vel.y * dt;
        }
    }
}

// ---------------------------------------------------------------------------
// Reusable actor movement helpers (shared by server and input/AI systems)
// ---------------------------------------------------------------------------

/// Step-march `origin` in `dir` up to `max_dist`, stopping before a wall hit.
/// Returns the farthest safe distance the actor can peek to.
pub fn clamped_peek_distance(
    origin: (f32, f32),
    dir: (f32, f32),
    max_dist: f32,
    half_size: f32,
    walls: &[Wall],
) -> f32 {
    const PEEK_STEP: f32 = px_to_tiles(0.5);
    let mut safe_dist = 0.0;
    let mut d = 0.0;
    while d < max_dist {
        d = (d + PEEK_STEP).min(max_dist);
        let cx = origin.0 + dir.0 * d;
        let cy = origin.1 + dir.1 * d;
        if walls.iter().any(|w| w.overlaps(cx, cy, half_size)) {
            break;
        }
        safe_dist = d;
    }
    safe_dist
}

/// Move an actor by a raw direction vector at a given speed, resolve wall
/// collisions, and clamp to level bounds.  Called by the server for each
/// connected session so that all physics math lives in `game`, not in the
/// server crate.
pub fn apply_actor_movement(
    x: &mut f32,
    y: &mut f32,
    dx: f32,
    dy: f32,
    speed: f32,
    dt: f32,
    walls: &[Wall],
    actor_half: f32,
    level_bounds: Option<LevelBounds>,
) {
    let len = (dx * dx + dy * dy).sqrt();
    let (ndx, ndy) = if len > 1.0 { (dx / len, dy / len) } else { (dx, dy) };
    *x += ndx * speed * dt;
    *y += ndy * speed * dt;
    wall::resolve_all(x, y, actor_half, walls);
    clamp_actor_to_level_bounds(x, y, actor_half, level_bounds);
}
