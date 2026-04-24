use crate::entity::enemy::Enemy;
use crate::entity::player::Player;
use engine::input::InputState;
use crate::spawn::SpawnQueue;
use crate::world::bounds::clamp_actor_to_level_bounds;
use crate::world::level::LevelBounds;
use crate::world::units::px_to_tiles;
use crate::world::wall::{self, Wall};

pub const PEEK_DISTANCE: f32 = px_to_tiles(18.0);
pub const PEEK_LERP_SPEED: f32 = 12.0;

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
/// collisions, and clamp to level bounds.
///
/// `dx`/`dy` are the unnormalised intent (e.g. `-1`, `0`, `1`). This function
/// normalises them so diagonal movement is not faster than axis-aligned movement.
/// Designed to be called by the server for each connected session so that all
/// physics math lives in `game`, not in the server crate.
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
    let (ndx, ndy) = if len > 1.0 {
        (dx / len, dy / len)
    } else {
        (dx, dy)
    };
    *x += ndx * speed * dt;
    *y += ndy * speed * dt;
    wall::resolve_all(x, y, actor_half, walls);
    clamp_actor_to_level_bounds(x, y, actor_half, level_bounds);
}

/// Apply input, resolve wall collisions, and clamp the player to the level bounds
/// in one atomic step so no caller needs to know the resolution order.
pub fn step_player(
    player: &mut Player,
    dt: f32,
    input: &InputState,
    walls: &[Wall],
    player_half: f32,
    level_bounds: Option<LevelBounds>,
    spawns: &mut SpawnQueue,
) {
    player.update(dt, input, walls, player_half, spawns);
    wall::resolve_all(
        &mut player.movement.x,
        &mut player.movement.y,
        player_half,
        walls,
    );
    clamp_actor_to_level_bounds(
        &mut player.movement.x,
        &mut player.movement.y,
        player_half,
        level_bounds,
    );
}

/// Run AI, move all enemies, resolve their wall collisions, and clamp them to
/// the level bounds.  The player position is passed so the AI can track it.
pub fn step_enemies(
    enemies: &mut [Enemy],
    dt: f32,
    player_pos: (f32, f32),
    walls: &[Wall],
    enemy_half: f32,
    level_bounds: Option<LevelBounds>,
    spawns: &mut SpawnQueue,
) {
    for enemy in enemies.iter_mut() {
        enemy.update(dt, player_pos, walls, spawns);
        wall::resolve_all(
            &mut enemy.movement.x,
            &mut enemy.movement.y,
            enemy_half,
            walls,
        );
        clamp_actor_to_level_bounds(
            &mut enemy.movement.x,
            &mut enemy.movement.y,
            enemy_half,
            level_bounds,
        );
    }
}
