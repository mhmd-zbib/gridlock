use crate::entity::enemy::Enemy;
use crate::entity::player::Player;
use crate::input::InputState;
use crate::spawn::SpawnQueue;
use crate::world::bounds::clamp_actor_to_level_bounds;
use crate::world::level::LevelBounds;
use crate::world::wall::{self, Wall};

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
