use crate::entity::bullet::{Bullet, BulletOwner};
use crate::entity::enemy::Enemy;
use crate::spawn::{SpawnQueue, SpawnRequest};
use crate::systems::projectile::ImpactEvent;
use crate::world::ray::cast_ray;
use crate::world::wall::Wall;

/// Materialise all queued spawn requests into live entities.
///
/// For player-owned bullets the complete trajectory is ray-cast to the first
/// wall before the bullet is inserted.  The resulting `ImpactEvent` is appended
/// to `bullet_traces` so the server can broadcast a `BulletEvent` packet in the
/// same tick the shot was fired — no need to wait for the projectile to
/// physically travel to its destination.
///
/// Enemy bullets are created without a trace because their hits are tracked
/// through the projectile system on both client and server.
pub fn flush_spawn_queue(
    queue: &mut SpawnQueue,
    bullets: &mut Vec<Bullet>,
    enemies: &mut Vec<Enemy>,
    walls: &[Wall],
    bullet_traces: &mut Vec<ImpactEvent>,
    bullet_max_range: f32,
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
                if matches!(owner, BulletOwner::Player) {
                    let dist = cast_ray((x, y), (dir_x, dir_y), bullet_max_range, walls);
                    bullet_traces.push(ImpactEvent {
                        x: x + dir_x * dist,
                        y: y + dir_y * dist,
                        origin_x: x,
                        origin_y: y,
                    });
                }
                bullets.push(Bullet::new(x, y, dir_x, dir_y, speed, damage, owner));
            }
            SpawnRequest::Enemy { x, y } => {
                enemies.push(Enemy::new(x, y));
            }
        }
    }
}
