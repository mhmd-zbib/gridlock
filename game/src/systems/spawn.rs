use crate::entity::bullet::Bullet;
use crate::entity::enemy::Enemy;
use crate::spawn::{SpawnQueue, SpawnRequest};

pub fn flush_spawn_queue(
    queue: &mut SpawnQueue,
    bullets: &mut Vec<Bullet>,
    enemies: &mut Vec<Enemy>,
) {
    for req in queue.drain() {
        match req {
            SpawnRequest::Bullet {
                x,
                y,
                dir_x,
                dir_y,
                speed,
                damage,
                owner,
            } => {
                bullets.push(Bullet::new(x, y, dir_x, dir_y, speed, damage, owner));
            }
            SpawnRequest::Enemy { x, y } => {
                enemies.push(Enemy::new(x, y));
            }
        }
    }
}
