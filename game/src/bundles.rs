use ecs::{Entity, World};

use crate::components::{
    AiAgent, BotTag, BulletData, BulletTag, CollisionRadius, Damage, Health, ImpactTag, Lifetime,
    PlayerController, PlayerTag, Position, Speed, VelocityFrac, Velocity, VisibilityState,
};
use crate::entity::bullet::BulletOwner;
use crate::entity::enemy::EnemyKind;
use crate::entity::player::PlayerLoadoutConfig;
use crate::game::{BULLET_MAX_RANGE, ENEMY_HALF, IMPACT_TTL, PLAYER_HALF};
use crate::world::sight::Sight;
use crate::world::sound::SoundField;
use crate::world::units::px_to_tiles;

// ---------------------------------------------------------------------------
// Player bundle
// ---------------------------------------------------------------------------

pub fn spawn_player(
    world: &mut World,
    x: f32,
    y: f32,
    loadout: &PlayerLoadoutConfig,
) -> Entity {
    let mut controller = PlayerController::new();
    controller.loadout.set_weapon(loadout.weapon);
    controller.loadout.set_attachments(loadout.attachments.clone());

    let stats = controller.loadout.active_stats();
    let mut sight = Sight::player();
    sight.range = stats.visibility_range;
    sight.half_angle = stats.visibility_half_angle_deg.to_radians();

    let entity = world.spawn();
    world.insert(entity, PlayerTag);
    world.insert(entity, Position { x, y });
    world.insert(entity, Speed(px_to_tiles(85.0)));
    world.insert(entity, VelocityFrac(0.0));
    world.insert(entity, CollisionRadius(PLAYER_HALF));
    world.insert(entity, Health::new(100));
    world.insert(entity, sight);
    world.insert(entity, SoundField::new());
    world.insert(entity, controller);
    world.insert(entity, VisibilityState::default());
    entity
}

// ---------------------------------------------------------------------------
// Enemy bundle
// ---------------------------------------------------------------------------

pub fn spawn_enemy(world: &mut World, x: f32, y: f32, kind: EnemyKind) -> Entity {
    let sight = Sight::enemy();
    let agent = AiAgent::new(x, y, sight.direction, kind);

    let entity = world.spawn();
    world.insert(entity, BotTag);
    world.insert(entity, Position { x, y });
    world.insert(entity, Speed(px_to_tiles(90.0)));
    world.insert(entity, VelocityFrac(0.0));
    world.insert(entity, CollisionRadius(ENEMY_HALF));
    world.insert(entity, Health::new(3));
    world.insert(entity, sight);
    world.insert(entity, agent);
    world.insert(entity, VisibilityState::default());
    entity
}

// ---------------------------------------------------------------------------
// Bullet bundle
// ---------------------------------------------------------------------------

pub fn spawn_bullet(
    world: &mut World,
    x: f32,
    y: f32,
    dir_x: f32,
    dir_y: f32,
    speed: f32,
    damage: u32,
    owner: BulletOwner,
) -> Entity {
    let entity = world.spawn();
    world.insert(entity, BulletTag);
    world.insert(entity, Position { x, y });
    world.insert(entity, Velocity { x: dir_x * speed, y: dir_y * speed });
    world.insert(entity, Damage(damage));
    world.insert(entity, BulletData { origin: (x, y), owner });
    world.insert(entity, Lifetime::new(BULLET_MAX_RANGE / speed));
    entity
}

// ---------------------------------------------------------------------------
// Impact bundle
// ---------------------------------------------------------------------------

pub fn spawn_impact(world: &mut World, x: f32, y: f32) -> Entity {
    let entity = world.spawn();
    world.insert(entity, ImpactTag);
    world.insert(entity, Position { x, y });
    world.insert(entity, Lifetime::new(IMPACT_TTL));
    entity
}
