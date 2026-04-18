use crate::entity::enemy::Enemy;
use crate::entity::player::Player;
use crate::input::InputState;
use crate::spawn::SpawnQueue;
use crate::world::bounds::clamp_actor_to_level_bounds;
use crate::world::level::LevelBounds;
use crate::world::wall::{self, Wall};

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
