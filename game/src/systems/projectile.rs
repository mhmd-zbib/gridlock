use crate::entity::bullet::{Bullet, BulletOwner};
use crate::entity::enemy::Enemy;
use crate::world::wall::{Wall, pierce_breakable_chain};

pub struct ImpactEvent {
    /// Impact position (where the bullet stopped).
    pub x: f32,
    pub y: f32,
    /// Muzzle position (where the bullet was spawned).
    pub origin_x: f32,
    pub origin_y: f32,
}

pub fn step_projectiles(
    dt: f32,
    bullets: &mut Vec<Bullet>,
    walls: &mut Vec<Wall>,
    enemies: &mut Vec<Enemy>,
    player_pos: (f32, f32),
    enemy_half: f32,
    player_half: f32,
) -> Vec<ImpactEvent> {
    let mut impacts = Vec::new();

    for bullet in bullets.iter_mut() {
        bullet.update(dt);
        if let Some(hit_idx) = walls.iter().position(|w| w.contains(bullet.x, bullet.y)) {
            let (impact_x, impact_y) = if walls[hit_idx].breakable {
                pierce_breakable_chain(walls, (bullet.x, bullet.y), bullet.dir(), bullet.damage)
            } else {
                (bullet.x, bullet.y)
            };
            bullet.alive = false;
            impacts.push(ImpactEvent {
                x: impact_x,
                y: impact_y,
                origin_x: bullet.origin_x,
                origin_y: bullet.origin_y,
            });
            continue;
        }

        match bullet.owner {
            BulletOwner::Player => {
                let hit_sq = (enemy_half * 2.0) * (enemy_half * 2.0);
                for enemy in enemies.iter_mut() {
                    let dx = bullet.x - enemy.movement.x;
                    let dy = bullet.y - enemy.movement.y;
                    if dx * dx + dy * dy < hit_sq {
                        bullet.alive = false;
                        enemy.hp = enemy.hp.saturating_sub(bullet.damage);
                        impacts.push(ImpactEvent {
                            x: bullet.x,
                            y: bullet.y,
                            origin_x: bullet.origin_x,
                            origin_y: bullet.origin_y,
                        });
                        break;
                    }
                }
            }
            BulletOwner::Enemy => {
                let dx = bullet.x - player_pos.0;
                let dy = bullet.y - player_pos.1;
                let hit_sq = (player_half * 2.0) * (player_half * 2.0);
                if dx * dx + dy * dy < hit_sq {
                    bullet.alive = false;
                    impacts.push(ImpactEvent {
                        x: bullet.x,
                        y: bullet.y,
                        origin_x: bullet.origin_x,
                        origin_y: bullet.origin_y,
                    });
                }
            }
        }
    }

    bullets.retain(|b| b.alive);
    impacts
}
