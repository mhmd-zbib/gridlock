use crate::core::entity::bullet::{Bullet, BulletOwner};
use crate::core::entity::enemy::Enemy;
use crate::core::world::wall::Wall;

pub struct ImpactEvent {
    pub x: f32,
    pub y: f32,
}

pub fn step_projectiles(
    dt: f32,
    bullets: &mut Vec<Bullet>,
    walls: &[Wall],
    enemies: &mut Vec<Enemy>,
    player_pos: (f32, f32),
    enemy_half: f32,
    player_half: f32,
) -> Vec<ImpactEvent> {
    let mut impacts = Vec::new();

    for bullet in bullets.iter_mut() {
        bullet.update(dt);
        if walls.iter().any(|w| w.contains(bullet.x, bullet.y)) {
            bullet.alive = false;
            impacts.push(ImpactEvent {
                x: bullet.x,
                y: bullet.y,
            });
            continue;
        }

        match bullet.owner {
            BulletOwner::Player => {
                for enemy in enemies.iter_mut() {
                    let dx = bullet.x - enemy.movement.x;
                    let dy = bullet.y - enemy.movement.y;
                    if (dx * dx + dy * dy).sqrt() < enemy_half * 2.0 {
                        bullet.alive = false;
                        enemy.hp = enemy.hp.saturating_sub(bullet.damage);
                        impacts.push(ImpactEvent {
                            x: bullet.x,
                            y: bullet.y,
                        });
                        break;
                    }
                }
            }
            BulletOwner::Enemy => {
                let dx = bullet.x - player_pos.0;
                let dy = bullet.y - player_pos.1;
                if (dx * dx + dy * dy).sqrt() < player_half * 2.0 {
                    bullet.alive = false;
                    impacts.push(ImpactEvent {
                        x: bullet.x,
                        y: bullet.y,
                    });
                }
            }
        }
    }

    bullets.retain(|b| b.alive);
    impacts
}
