use crate::core::entity::enemy::Enemy;
use crate::core::world::sight::Sight;
use crate::core::world::wall::Wall;

pub fn sync_enemy_visibility(
    enemies: &mut [Enemy],
    player_pos: (f32, f32),
    player_sight: &Sight,
    walls: &[Wall],
) {
    for enemy in enemies {
        let ep = (enemy.movement.x, enemy.movement.y);
        enemy.visible_to_player = player_sight.can_see(player_pos, ep, walls);
    }
}
