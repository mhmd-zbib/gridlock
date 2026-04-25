use ecs::{Entity, World};
use engine::input::InputState;

use crate::bundles;
use crate::components::{
    AiAgent, BotTag, BulletTag, ImpactTag, PlayerController, Position,
    VisibilityState, Health,
};
use crate::entity::enemy::EnemyKind;
use crate::entity::player::PlayerLoadoutConfig;
use crate::events::BulletTraceEvent;
use crate::systems::{ai, combat, input, lifetime, movement, physics, projectile, sound, spawn, visibility};
use crate::world::floor::{self as floor_world, ResolvedFloor};
use crate::world::level::{LevelBounds, LevelData};
use crate::world::light::LightSource;
use crate::world::prop::{self, ResolvedProp};
use crate::world::rooms::LevelRooms;
use crate::world::sight::Sight;
use crate::world::sound::SoundField;
use crate::world::units::px_to_tiles;
use crate::world::wall::Wall;
use crate::world::aim_cone::AimCone;
use crate::world::bounds::clamp_actor_to_level_bounds;

// ---------------------------------------------------------------------------
// Domain constants — authoritative sizes used by movement and combat systems
// ---------------------------------------------------------------------------

pub const PLAYER_HALF: f32 = px_to_tiles(10.0);
pub const ENEMY_HALF: f32 = px_to_tiles(8.0);
pub const BULLET_MAX_RANGE: f32 = px_to_tiles(1500.0);
pub const IMPACT_TTL: f32 = 0.15;

// ---------------------------------------------------------------------------
// View types (returned to callers for rendering / network snapshots)
// ---------------------------------------------------------------------------

/// Read-only snapshot of the player entity for the current frame.
pub struct PlayerView {
    pub x: f32,
    pub y: f32,
    pub sight: Sight,
    pub aim_cone: AimCone,
    pub sound_field_smoothed_radius: f32,
}

/// Read-only snapshot of one enemy entity used by the renderer.
pub struct EnemyView {
    pub entity: Entity,
    pub x: f32,
    pub y: f32,
    pub sight: Sight,
    pub kind: EnemyKind,
    pub hp: u32,
    pub visible_to_player: bool,
    // AI state for rendering
    pub ai_awareness: crate::ai::awareness::AiState,
    pub ai_suspicion: f32,
    pub ai_in_combat: bool,
    pub ai_is_alert: bool,
    pub ai_spawn_anchor: (f32, f32),
    pub ai_last_known_pos: Option<(f32, f32)>,
    pub ai_last_move_target: Option<(f32, f32)>,
    pub ai_phase_name: &'static str,
    pub ai_gap_waypoints: Vec<(f32, f32)>,
}

/// Position + fade fraction for an impact mark (used by renderer).
#[derive(Clone, Copy)]
pub struct ImpactView {
    pub x: f32,
    pub y: f32,
    /// Fade alpha in [0, 1] — 1.0 = just spawned, 0.0 = expired.
    pub alpha: f32,
}

// ---------------------------------------------------------------------------
// Game
// ---------------------------------------------------------------------------

/// Top-level game state backed by an ECS `World`.
///
/// Owns all entity data and drives the system pipeline each tick.
/// Callers (client, server, tests) interact only through the public API;
/// ECS internals are not exposed.
pub struct Game {
    world: World,
    player_entity: Entity,
    walls: Vec<Wall>,
    floors: Vec<ResolvedFloor>,
    props: Vec<ResolvedProp>,
    rooms: LevelRooms,
    level_bounds: Option<LevelBounds>,
    lights: Vec<LightSource>,
    player_loadout: PlayerLoadoutConfig,
}

impl Game {
    pub fn new() -> Self {
        let player_loadout = PlayerLoadoutConfig::default();
        let mut world = World::new();

        let player_entity = bundles::spawn_player(
            &mut world,
            px_to_tiles(400.0),
            px_to_tiles(300.0),
            &player_loadout,
        );
        // Two default enemies for single-player.
        bundles::spawn_enemy(&mut world, px_to_tiles(100.0), px_to_tiles(100.0), EnemyKind::Shooter);
        bundles::spawn_enemy(&mut world, px_to_tiles(700.0), px_to_tiles(500.0), EnemyKind::Shooter);

        Self {
            world,
            player_entity,
            walls: Vec::new(),
            floors: Vec::new(),
            props: Vec::new(),
            rooms: LevelRooms::default(),
            level_bounds: None,
            lights: Vec::new(),
            player_loadout,
        }
    }

    pub fn set_player_loadout(&mut self, loadout: PlayerLoadoutConfig) {
        let mut sanitized = loadout;
        sanitized.sanitize();
        self.player_loadout = sanitized.clone();

        if let Some(ctrl) = self.world.get_mut::<PlayerController>(self.player_entity) {
            ctrl.loadout.set_weapon(sanitized.weapon);
            ctrl.loadout.set_attachments(sanitized.attachments);
            let stats = ctrl.loadout.active_stats();
            if let Some(sight) = self.world.get_mut::<Sight>(self.player_entity) {
                sight.range = stats.visibility_range;
                sight.half_angle = stats.visibility_half_angle_deg.to_radians();
            }
        }
    }

    pub fn player_loadout(&self) -> PlayerLoadoutConfig {
        self.player_loadout.clone()
    }

    pub fn player_weapon_catalog_index(&self) -> usize {
        self.player_loadout.weapon.catalog_index()
    }

    pub fn load_level(&mut self, level: &LevelData, level_width: f32, level_height: f32) {
        self.player_loadout.sanitize();

        // Clear all entities.
        let all: Vec<_> = self.world.all_entities();
        for e in all {
            self.world.despawn(e);
        }

        // Spawn player at level spawn point.
        let (px, py) = level
            .player_spawn
            .map(|sp| (sp.x, sp.y))
            .unwrap_or((px_to_tiles(400.0), px_to_tiles(300.0)));
        self.player_entity = bundles::spawn_player(&mut self.world, px, py, &self.player_loadout);

        // Spawn enemies.
        for p in &level.enemies {
            bundles::spawn_enemy(&mut self.world, p.x, p.y, EnemyKind::Shooter);
        }
        for p in &level.target_enemies {
            bundles::spawn_enemy(&mut self.world, p.x, p.y, EnemyKind::TargetDummy);
        }

        // Load static world data.
        let floor_assets = floor_world::load_assets();
        self.floors = floor_world::resolve_level_floors(&level.floors, &floor_assets);
        let prop_assets = prop::load_assets();
        self.props = prop::resolve_level_props(&level.props, &prop_assets);
        self.walls = build_walls(&level.walls, &self.props);
        self.lights = level.lights.iter().map(|l| l.clone().into_source()).collect();
        self.level_bounds = level.map_bounds;

        // Clamp spawned entities to level bounds.
        let bounds = self.level_bounds;
        let entities: Vec<_> = self.world.entities_with::<CollisionRadius>();
        for entity in entities {
            let r = self.world.get::<CollisionRadius>(entity).copied().unwrap_or_default().0;
            if let Some(pos) = self.world.get_mut::<Position>(entity) {
                clamp_actor_to_level_bounds(&mut pos.x, &mut pos.y, r, bounds);
            }
        }

        let room_w = level.map_bounds.map_or(level_width, |b| (b.x + b.w).max(level_width));
        let room_h = level.map_bounds.map_or(level_height, |b| (b.y + b.h).max(level_height));
        self.rooms = LevelRooms::compute(&self.walls, room_w, room_h);
    }

    /// Drain bullet traces accumulated during the last `update()`.
    pub fn take_bullet_traces(&mut self) -> Vec<BulletTraceEvent> {
        self.world.events.drain::<BulletTraceEvent>()
    }

    /// Advance the game by one fixed timestep.
    ///
    /// Execution order:
    /// 1. Input system
    /// 2. AI system
    /// 3. Movement system (bullet translation)
    /// 4. Physics system (wall collision + bounds)
    /// 5. Visibility system
    /// 6. Sound system
    /// 7. Projectile system (hit detection)
    /// 8. Combat system (apply damage)
    /// 9. Lifetime system (expiry)
    /// 10. Spawn system (materialise bullet/enemy events)
    /// 11. Despawn system (remove dead entities)
    pub fn update(&mut self, dt: f32, input: &InputState) {
        let walls = self.walls.clone();
        let bounds = self.level_bounds;

        let player_pos = self
            .world
            .get::<Position>(self.player_entity)
            .copied()
            .map(|p| (p.x, p.y))
            .unwrap_or((0.0, 0.0));

        input::run(&mut self.world, input, &walls, dt);
        ai::run(&mut self.world, player_pos, &walls, dt);
        movement::run(&mut self.world, dt);
        physics::run(&mut self.world, &walls, bounds);
        visibility::run(&mut self.world, &walls);
        sound::run(&mut self.world, dt);
        projectile::run(&mut self.world, &mut self.walls);
        combat::run(&mut self.world);
        lifetime::run(&mut self.world, dt);

        for light in &mut self.lights {
            light.tick(dt);
        }

        spawn::run(&mut self.world, &walls);
        spawn::despawn(&mut self.world);
    }

    // -----------------------------------------------------------------------
    // Read-only accessors for rendering and networking
    // -----------------------------------------------------------------------

    /// Player's current position.
    pub fn player_pos(&self) -> (f32, f32) {
        self.world
            .get::<Position>(self.player_entity)
            .map(|p| (p.x, p.y))
            .unwrap_or((0.0, 0.0))
    }

    /// Current sight cone configuration for the player.
    pub fn player_sight(&self) -> Sight {
        self.world
            .get::<Sight>(self.player_entity)
            .copied()
            .unwrap_or_else(Sight::player)
    }

    /// Current aim cone for the player.
    pub fn player_aim_cone(&self) -> AimCone {
        self.world
            .get::<PlayerController>(self.player_entity)
            .map(|c| c.aim_cone.clone())
            .unwrap_or_else(AimCone::new)
    }

    /// Active sound field for the player.
    pub fn player_sound_field(&self) -> Option<&SoundField> {
        self.world.get::<SoundField>(self.player_entity)
    }

    /// Active weapon stats for the player.
    pub fn player_weapon_stats(&self) -> crate::entity::weapon::WeaponStats {
        self.world
            .get::<PlayerController>(self.player_entity)
            .map(|c| c.loadout.active_stats())
            .unwrap_or_default()
    }

    pub fn player_ammo_in_mag(&self) -> u32 {
        self.world
            .get::<PlayerController>(self.player_entity)
            .map(|c| c.loadout.ammo_in_mag())
            .unwrap_or(0)
    }

    pub fn player_mag_size(&self) -> u32 {
        self.world
            .get::<PlayerController>(self.player_entity)
            .map(|c| c.loadout.mag_size())
            .unwrap_or(0)
    }

    pub fn player_is_reloading(&self) -> bool {
        self.world
            .get::<PlayerController>(self.player_entity)
            .map(|c| c.loadout.is_reloading())
            .unwrap_or(false)
    }

    pub fn player_weapon_name(&self) -> &'static str {
        self.world
            .get::<PlayerController>(self.player_entity)
            .map(|c| c.loadout.weapon_name())
            .unwrap_or("")
    }

    pub fn player_weapon_class_label(&self) -> &'static str {
        self.world
            .get::<PlayerController>(self.player_entity)
            .map(|c| c.loadout.weapon_class_label())
            .unwrap_or("")
    }

    pub fn player_attachment_name_for(
        &self,
        category: crate::entity::weapon::attachment::AttachmentCategory,
    ) -> Option<&'static str> {
        self.world
            .get::<PlayerController>(self.player_entity)
            .and_then(|c| c.loadout.attachment_name_for(category))
    }

    /// All enemy views (position, sight, HP, AI state) for the current frame.
    pub fn enemy_views(&self) -> Vec<EnemyView> {
        self.world
            .entities_with::<BotTag>()
            .into_iter()
            .filter_map(|entity| {
                let pos = self.world.get::<Position>(entity).copied()?;
                let sight = self.world.get::<Sight>(entity).copied()?;
                let health = self.world.get::<Health>(entity).copied()?;
                let agent = self.world.get::<AiAgent>(entity)?;
                let vis = self.world.get::<VisibilityState>(entity).copied().unwrap_or_default();
                let ep = (pos.x, pos.y);
                Some(EnemyView {
                    entity,
                    x: pos.x,
                    y: pos.y,
                    sight,
                    kind: agent.kind,
                    hp: health.current,
                    visible_to_player: vis.is_visible(),
                    ai_awareness: agent.brain.awareness.state,
                    ai_suspicion: agent.brain.awareness.suspicion,
                    ai_in_combat: agent.brain.awareness.in_combat(),
                    ai_is_alert: agent.brain.awareness.is_alert(),
                    ai_spawn_anchor: agent.brain.spawn_anchor(),
                    ai_last_known_pos: agent.brain.awareness.last_known_pos(),
                    ai_last_move_target: agent.brain.last_move_target,
                    ai_phase_name: agent.brain.phase_name(),
                    ai_gap_waypoints: agent.brain.debug_gaps(ep, &self.walls),
                })
            })
            .collect()
    }

    /// Impact mark positions with fade alpha for the current frame.
    pub fn impact_views(&self) -> Vec<ImpactView> {
        use crate::components::Lifetime;
        self.world
            .entities_with::<ImpactTag>()
            .into_iter()
            .filter_map(|e| {
                let pos = self.world.get::<Position>(e).copied()?;
                let lt = self.world.get::<Lifetime>(e).copied()?;
                Some(ImpactView {
                    x: pos.x,
                    y: pos.y,
                    alpha: (lt.remaining / IMPACT_TTL).clamp(0.0, 1.0),
                })
            })
            .collect()
    }

    /// Positions of all live bullets.
    pub fn bullet_positions(&self) -> Vec<(f32, f32)> {
        self.world
            .entities_with::<BulletTag>()
            .into_iter()
            .filter_map(|e| self.world.get::<Position>(e).map(|p| (p.x, p.y)))
            .collect()
    }

    /// Positions of all live impact marks.
    pub fn impact_positions(&self) -> Vec<(f32, f32)> {
        self.world
            .entities_with::<ImpactTag>()
            .into_iter()
            .filter_map(|e| self.world.get::<Position>(e).map(|p| (p.x, p.y)))
            .collect()
    }

    pub fn walls(&self) -> &[Wall] {
        &self.walls
    }

    pub fn walls_mut(&mut self) -> &mut Vec<Wall> {
        &mut self.walls
    }

    pub fn floors(&self) -> &[ResolvedFloor] {
        &self.floors
    }

    pub fn props(&self) -> &[ResolvedProp] {
        &self.props
    }

    pub fn lights(&self) -> &[LightSource] {
        &self.lights
    }

    pub fn rooms(&self) -> &LevelRooms {
        &self.rooms
    }

    pub fn level_bounds(&self) -> Option<LevelBounds> {
        self.level_bounds
    }

    /// Direct ECS World access (read-only) for advanced queries.
    pub fn world(&self) -> &World {
        &self.world
    }

    /// Direct ECS World access (mutable) — primarily for tests.
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    /// Player entity ID (for direct ECS queries).
    pub fn player_entity(&self) -> Entity {
        self.player_entity
    }

    /// Set the player's facing direction (used in tests).
    pub fn set_player_sight_direction(&mut self, direction: f32) {
        if let Some(sight) = self.world.get_mut::<Sight>(self.player_entity) {
            sight.direction = direction;
        }
        if let Some(ctrl) = self.world.get_mut::<PlayerController>(self.player_entity) {
            ctrl.aim_cone.direction = direction;
        }
    }

    /// Number of live enemy entities.
    pub fn enemy_count(&self) -> usize {
        self.world.entities_with::<BotTag>().len()
    }
}

// ---------------------------------------------------------------------------
// Level-load helpers
// ---------------------------------------------------------------------------

fn build_walls(level_walls: &[Wall], props: &[ResolvedProp]) -> Vec<Wall> {
    let mut walls: Vec<Wall> = level_walls.to_vec();
    for wall in &mut walls {
        wall.init_segments();
    }
    for prop in props.iter().filter(|p| p.is_collider) {
        let mut w = Wall::new(
            prop.x - prop.width * 0.5,
            prop.y - prop.height * 0.5,
            prop.width,
            prop.height,
        );
        w.is_prop = true;
        walls.push(w);
    }
    walls
}

// Bring CollisionRadius into scope for the load_level impl.
use crate::components::CollisionRadius;
