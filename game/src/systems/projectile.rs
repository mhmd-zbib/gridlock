use ecs::World;

use crate::components::{
    BotTag, BulletData, BulletTag, CollisionRadius, Damage, PlayerTag, Position,
};
use crate::entity::bullet::BulletOwner;
use crate::events::{BulletTraceEvent, DamageEvent, DestroyEvent};
use crate::world::wall::{Wall, pierce_breakable_chain};

/// Runs one tick of bullet physics: checks for wall and entity hits, and
/// emits the appropriate events.  Does not remove any entities directly —
/// `DespawnSystem` handles that by consuming `DestroyEvent`.
///
/// Type-agnostic: operates on any entity with `BulletTag + Position + Damage + BulletData`.
pub fn run(world: &mut World, walls: &mut Vec<Wall>) {
    let bullets: Vec<_> = world.entities_with::<BulletTag>();

    // Collect target positions for hit detection.
    let bots: Vec<_> = world
        .entities_with::<BotTag>()
        .into_iter()
        .filter_map(|e| {
            let pos = world.get::<Position>(e).copied()?;
            let r = world.get::<CollisionRadius>(e).copied()?.0;
            Some((e, pos, r))
        })
        .collect();

    let players: Vec<_> = world
        .entities_with::<PlayerTag>()
        .into_iter()
        .filter_map(|e| {
            let pos = world.get::<Position>(e).copied()?;
            let r = world.get::<CollisionRadius>(e).copied()?.0;
            Some((e, pos, r))
        })
        .collect();

    for bullet in bullets {
        let pos = match world.get::<Position>(bullet).copied() {
            Some(p) => p,
            None => continue,
        };
        let data = match world.get::<BulletData>(bullet).copied() {
            Some(d) => d,
            None => continue,
        };
        let dmg = world.get::<Damage>(bullet).copied().unwrap_or(Damage(1)).0;

        // Wall hit.
        if let Some(hit_idx) = walls.iter().position(|w| w.contains(pos.x, pos.y)) {
            let (impact_x, impact_y) = if walls[hit_idx].breakable {
                pierce_breakable_chain(walls, (pos.x, pos.y), bullet_dir(world, bullet), dmg)
            } else {
                (pos.x, pos.y)
            };

            emit_trace_if_player_bullet(world, &data, impact_x, impact_y);
            world.events.emit(DestroyEvent { entity: bullet });
            continue;
        }

        // Entity hits — player bullets hit bots, enemy bullets hit players.
        match data.owner {
            BulletOwner::Player => {
                if let Some(&(target, _, _)) =
                    bots.iter().find(|(_, tp, r)| hit_check(pos, *tp, *r))
                {
                    world.events.emit(DamageEvent { target, amount: dmg, source: bullet });
                    world.events.emit(BulletTraceEvent {
                        x: pos.x,
                        y: pos.y,
                        origin_x: data.origin.0,
                        origin_y: data.origin.1,
                    });
                    world.events.emit(DestroyEvent { entity: bullet });
                }
            }
            BulletOwner::Enemy => {
                if let Some(&(target, _, _)) =
                    players.iter().find(|(_, tp, r)| hit_check(pos, *tp, *r))
                {
                    world.events.emit(DamageEvent { target, amount: dmg, source: bullet });
                    world.events.emit(DestroyEvent { entity: bullet });
                }
            }
        }
    }
}

fn hit_check(bullet_pos: Position, target_pos: Position, target_radius: f32) -> bool {
    let dx = bullet_pos.x - target_pos.x;
    let dy = bullet_pos.y - target_pos.y;
    let threshold = target_radius * 2.0;
    dx * dx + dy * dy < threshold * threshold
}

fn bullet_dir(world: &World, bullet: ecs::Entity) -> (f32, f32) {
    use crate::components::Velocity;
    let vel = world.get::<Velocity>(bullet).copied().unwrap_or_default();
    let len = (vel.x * vel.x + vel.y * vel.y).sqrt();
    if len > 1e-9 { (vel.x / len, vel.y / len) } else { (0.0, 0.0) }
}

fn emit_trace_if_player_bullet(
    world: &mut World,
    data: &BulletData,
    impact_x: f32,
    impact_y: f32,
) {
    if data.owner == BulletOwner::Player {
        world.events.emit(BulletTraceEvent {
            x: impact_x,
            y: impact_y,
            origin_x: data.origin.0,
            origin_y: data.origin.1,
        });
    }
}
