use ecs::World;
use engine::input::InputState;

use crate::components::{
    CollisionRadius, PlayerController, PlayerTag, Position, Speed, VelocityFrac,
};
use crate::entity::bullet::BulletOwner;
use crate::entity::movement::Movement;
use crate::events::SpawnBulletEvent;
use crate::world::sight::Sight;
use crate::world::units::px_to_tiles;
use crate::world::wall::Wall;
use crate::entity::player::locomotion::RUN_SPEED;

/// Process player input: locomotion, aiming, weapon tick, and shooting.
///
/// Queries entities with `PlayerTag` only.  Emits `SpawnBulletEvent` for each
/// round fired instead of pushing directly to a spawn queue.
pub fn run(world: &mut World, input: &InputState, walls: &[Wall], dt: f32) {
    let players: Vec<_> = world.entities_with::<PlayerTag>();
    for entity in players {
        step_player(world, entity, input, walls, dt);
    }
}

fn step_player(
    world: &mut World,
    entity: ecs::Entity,
    input: &InputState,
    walls: &[Wall],
    dt: f32,
) {
    // Take ownership of mutable components to avoid simultaneous borrows.
    let mut controller = match world.remove::<PlayerController>(entity) {
        Some(c) => c,
        None => return,
    };
    let mut pos = world.get::<Position>(entity).copied().unwrap_or_default();
    let radius = world.get::<CollisionRadius>(entity).copied().unwrap_or_default();

    // Build a transient Movement so locomotion code can operate unchanged.
    let speed = world.get::<Speed>(entity).copied().unwrap_or(Speed(px_to_tiles(85.0))).0;
    let vel_frac = world.get::<VelocityFrac>(entity).copied().unwrap_or_default().0;
    let mut movement = Movement { x: pos.x, y: pos.y, speed, velocity_frac: vel_frac };

    controller.locomotion.step(dt, input, &mut movement, radius.0, walls);

    pos.x = movement.x;
    pos.y = movement.y;

    // Aim toward mouse.
    let mouse_world = (px_to_tiles(input.mouse_x as f32), px_to_tiles(input.mouse_y as f32));
    let mut sight = world.remove::<Sight>(entity).unwrap_or_else(Sight::player);
    sight.face((pos.x, pos.y), mouse_world, dt);
    controller.aim_cone.direction = sight.direction;

    // Tick weapon, handle reload input and auto-reload on empty.
    controller.loadout.tick(dt);
    controller.loadout.update_reload_input(input.reload);
    controller.loadout.auto_reload_if_dry_trigger(input.shoot);

    // Update sight range and spread from active weapon stats.
    let stats = controller.loadout.active_stats();
    sight.range = stats.visibility_range;
    sight.half_angle = stats.visibility_half_angle_deg.to_radians();
    controller.aim_cone.set_spread_profile(
        stats.aim_base_half_angle_deg,
        stats.movement_spread_max_deg,
    );

    let speed_frac = movement.velocity_frac * (movement.speed / RUN_SPEED);
    controller.aim_cone.update(dt, speed_frac, stats.recoil_decay_deg_per_sec);

    // Fire.
    if input.shoot && controller.loadout.try_fire() {
        let (dir_x, dir_y) = controller.aim_cone.sample_direction();
        world.events.emit(SpawnBulletEvent {
            x: pos.x,
            y: pos.y,
            dir_x,
            dir_y,
            speed: stats.bullet_speed,
            damage: stats.bullet_damage,
            owner: BulletOwner::Player,
        });
        controller.aim_cone.on_shot(stats.recoil_per_shot_deg, stats.recoil_max_deg);
    }

    // Write back.
    world.insert(entity, pos);
    world.insert(entity, Speed(movement.speed));
    world.insert(entity, VelocityFrac(movement.velocity_frac));
    world.insert(entity, sight);
    world.insert(entity, controller);
}
