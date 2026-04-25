use ecs::World;

use crate::components::{AiAgent, BotTag, CollisionRadius, Position, Speed, VelocityFrac};
use crate::entity::bullet::BulletOwner;
use crate::entity::enemy::EnemyKind;
use crate::events::SpawnBulletEvent;
use crate::world::sight::Sight;
use crate::world::units::px_to_tiles;
use crate::world::wall::{self, Wall};

const ENEMY_BULLET_SPEED: f32 = px_to_tiles(600.0);
const ENEMY_BULLET_DAMAGE: u32 = 1;

/// Runs the AI brain for every bot entity and applies the resulting movement
/// and shoot decisions.  Type-agnostic: any entity with `BotTag + AiAgent`
/// goes through this system regardless of specific entity kind.
pub fn run(world: &mut World, player_pos: (f32, f32), walls: &[Wall], dt: f32) {
    let bots: Vec<_> = world.entities_with::<BotTag>();
    for entity in bots {
        step_bot(world, entity, player_pos, walls, dt);
    }
}

fn step_bot(
    world: &mut World,
    entity: ecs::Entity,
    player_pos: (f32, f32),
    walls: &[Wall],
    dt: f32,
) {
    let mut agent = match world.remove::<AiAgent>(entity) {
        Some(a) => a,
        None => return,
    };

    if agent.kind == EnemyKind::TargetDummy {
        world.insert(entity, VelocityFrac(0.0));
        world.insert(entity, agent);
        return;
    }

    let pos = world.get::<Position>(entity).copied().unwrap_or_default();
    let from = (pos.x, pos.y);

    let mut sight = world.remove::<Sight>(entity).unwrap_or_else(Sight::enemy);
    let output = agent.brain.update(from, &sight, player_pos, walls, dt);
    sight.direction = output.look_dir;

    let speed = world.get::<Speed>(entity).copied().unwrap_or(Speed(px_to_tiles(90.0))).0;
    let half = world.get::<CollisionRadius>(entity).copied().unwrap_or_default().0;

    let (new_pos, vel_frac) = if let Some(target) = output.move_target {
        move_toward(from, target, speed * dt, half, walls)
    } else {
        (from, 0.0)
    };

    if let Some(shoot_dir) = output.shoot {
        world.events.emit(SpawnBulletEvent {
            x: from.0,
            y: from.1,
            dir_x: shoot_dir.0,
            dir_y: shoot_dir.1,
            speed: ENEMY_BULLET_SPEED,
            damage: ENEMY_BULLET_DAMAGE,
            owner: BulletOwner::Enemy,
        });
    }

    world.insert(entity, Position { x: new_pos.0, y: new_pos.1 });
    world.insert(entity, VelocityFrac(vel_frac));
    world.insert(entity, sight);
    world.insert(entity, agent);
}

// ---------------------------------------------------------------------------
// Movement helpers (formerly in entity/enemy.rs)
// ---------------------------------------------------------------------------

fn move_toward(
    from: (f32, f32),
    target: (f32, f32),
    max_step: f32,
    half: f32,
    walls: &[Wall],
) -> ((f32, f32), f32) {
    let dx = target.0 - from.0;
    let dy = target.1 - from.1;
    let dist = dx.hypot(dy);
    if dist < px_to_tiles(1.0) {
        return (from, 0.0);
    }
    let step = max_step.min(dist);
    let nx = dx / dist * step;
    let ny = dy / dist * step;
    apply_move_with_slide(from, nx, ny, step, half, walls)
}

fn apply_move_with_slide(
    from: (f32, f32),
    nx: f32,
    ny: f32,
    step: f32,
    half: f32,
    walls: &[Wall],
) -> ((f32, f32), f32) {
    let (mut cx, mut cy) = (from.0 + nx, from.1 + ny);
    wall::resolve_all(&mut cx, &mut cy, half, walls);
    let diag_progress = (cx - from.0).hypot(cy - from.1);
    if diag_progress >= step * 0.5 {
        return ((cx, cy), (diag_progress / step).min(1.0));
    }

    let (mut sx, mut sy) = (from.0 + nx, from.1);
    wall::resolve_all(&mut sx, &mut sy, half, walls);
    let x_progress = (sx - from.0).abs();

    let (mut qx, mut qy) = (from.0, from.1 + ny);
    wall::resolve_all(&mut qx, &mut qy, half, walls);
    let y_progress = (qy - from.1).abs();

    if x_progress >= y_progress && x_progress > px_to_tiles(0.5) {
        ((sx, sy), (x_progress / step).min(1.0))
    } else if y_progress > px_to_tiles(0.5) {
        ((qx, qy), (y_progress / step).min(1.0))
    } else {
        (from, 0.0)
    }
}
