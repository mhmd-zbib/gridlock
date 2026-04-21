use super::entity::bullet::Bullet;
use super::entity::enemy::Enemy;
use super::entity::player::{Player, PlayerLoadoutConfig, sanitize_loadout};
use super::spawn::SpawnQueue;
pub use super::systems::projectile::ImpactEvent;
use super::systems::{movement, projectile, spawn as spawn_system, visibility};
use super::world::floor::{self as floor_world, ResolvedFloor};
use super::world::level::{LevelBounds, LevelData};
use super::world::prop::{self, ResolvedProp};
use super::world::rooms::LevelRooms;
use super::world::units::px_to_tiles;
use super::world::wall::Wall;
use crate::input::InputState;

// ---------------------------------------------------------------------------
// Domain constants — authoritative sizes used by movement and combat systems
// ---------------------------------------------------------------------------

/// Half-extent of the player collision circle in tile-space.
pub const PLAYER_HALF: f32 = px_to_tiles(10.0);
/// Half-extent of an enemy collision circle in tile-space.
pub const ENEMY_HALF: f32 = px_to_tiles(8.0);
/// Maximum distance a bullet can travel before it is discarded.
pub const BULLET_MAX_RANGE: f32 = px_to_tiles(1500.0);

pub const IMPACT_TTL: f32 = 0.15;

// ---------------------------------------------------------------------------
// ImpactMark
// ---------------------------------------------------------------------------

/// A short-lived visual marker placed at a bullet impact point.
pub struct ImpactMark {
    pub x: f32,
    pub y: f32,
    pub ttl: f32,
}

impl ImpactMark {
    fn new(x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            ttl: IMPACT_TTL,
        }
    }
}

// ---------------------------------------------------------------------------
// Game
// ---------------------------------------------------------------------------

/// Top-level game state.  Acts as the ECS "world": it owns every entity and
/// drives them by dispatching into focused systems each tick.
///
/// Game logic lives in the `systems` module; this struct is the orchestrator.
pub struct Game {
    pub player: Player,
    pub enemies: Vec<Enemy>,
    pub bullets: Vec<Bullet>,
    pub impacts: Vec<ImpactMark>,
    pub walls: Vec<Wall>,
    pub floors: Vec<ResolvedFloor>,
    pub props: Vec<ResolvedProp>,
    pub rooms: LevelRooms,
    pub level_bounds: Option<LevelBounds>,
    spawn_queue: SpawnQueue,
    player_loadout: PlayerLoadoutConfig,
    /// Bullet traces resolved this tick (origin → impact).
    ///
    /// Consumed by callers that need to report hits over the network.
    /// Cleared at the start of each `update()` call.
    bullet_traces: Vec<ImpactEvent>,
}

impl Game {
    pub fn new() -> Self {
        let player_loadout = PlayerLoadoutConfig::default();
        let mut player = Player::new(px_to_tiles(400.0), px_to_tiles(300.0));
        player.apply_loadout(&player_loadout);
        Self {
            player,
            enemies: vec![
                Enemy::new(px_to_tiles(100.0), px_to_tiles(100.0)),
                Enemy::new(px_to_tiles(700.0), px_to_tiles(500.0)),
            ],
            bullets: Vec::new(),
            impacts: Vec::new(),
            walls: Vec::new(),
            floors: Vec::new(),
            props: Vec::new(),
            rooms: LevelRooms::default(),
            level_bounds: None,
            spawn_queue: SpawnQueue::default(),
            player_loadout,
            bullet_traces: Vec::new(),
        }
    }

    pub fn set_player_loadout(&mut self, loadout: PlayerLoadoutConfig) {
        let mut sanitized = loadout;
        sanitize_loadout(&mut sanitized);
        self.player_loadout = sanitized;
        self.player.apply_loadout(&self.player_loadout);
    }

    pub fn player_loadout(&self) -> PlayerLoadoutConfig {
        self.player_loadout.clone()
    }

    /// Catalog index of the player's currently selected weapon.
    /// Matches `WeaponId::catalog_index()` — safe to cast to `u8` for the wire.
    pub fn player_weapon_catalog_index(&self) -> usize {
        self.player_loadout.weapon.catalog_index()
    }

    pub fn load_level(&mut self, level: &LevelData, level_width: f32, level_height: f32) {
        self.player = match level.player_spawn {
            Some(sp) => Player::new(sp.x, sp.y),
            None => Player::new(px_to_tiles(400.0), px_to_tiles(300.0)),
        };
        sanitize_loadout(&mut self.player_loadout);
        self.player.apply_loadout(&self.player_loadout);

        self.enemies = level
            .enemies
            .iter()
            .map(|p| Enemy::new(p.x, p.y))
            .chain(
                level
                    .target_enemies
                    .iter()
                    .map(|p| Enemy::target_dummy(p.x, p.y)),
            )
            .collect();

        let floor_assets = floor_world::load_assets();
        self.floors = floor_world::resolve_level_floors(&level.floors, &floor_assets);
        let prop_assets = prop::load_assets();
        self.props = prop::resolve_level_props(&level.props, &prop_assets);
        self.walls = level.walls.clone();
        for wall in &mut self.walls {
            wall.init_segments();
        }
        // Promote collidable props to walls so movement and combat systems share
        // a single wall list.
        for prop in self.props.iter().filter(|p| p.is_collider) {
            let mut w = Wall::new(
                prop.x - prop.width * 0.5,
                prop.y - prop.height * 0.5,
                prop.width,
                prop.height,
            );
            w.is_prop = true;
            self.walls.push(w);
        }

        self.bullets = Vec::new();
        self.impacts = Vec::new();
        self.spawn_queue = SpawnQueue::default();
        self.bullet_traces = Vec::new();
        self.level_bounds = level.map_bounds;

        // Clamp initial positions inside the level.
        use super::world::bounds::clamp_actor_to_level_bounds;
        clamp_actor_to_level_bounds(
            &mut self.player.movement.x,
            &mut self.player.movement.y,
            PLAYER_HALF,
            self.level_bounds,
        );
        for enemy in &mut self.enemies {
            clamp_actor_to_level_bounds(
                &mut enemy.movement.x,
                &mut enemy.movement.y,
                ENEMY_HALF,
                self.level_bounds,
            );
        }

        // Compute room / gap topology once at level load and cache it.
        let room_w = match level.map_bounds {
            Some(bounds) => (bounds.x + bounds.w).max(level_width),
            None => level_width,
        };
        let room_h = match level.map_bounds {
            Some(bounds) => (bounds.y + bounds.h).max(level_height),
            None => level_height,
        };
        self.rooms = LevelRooms::compute(&self.walls, room_w, room_h);
    }

    /// Drain the bullet traces accumulated during the last `update()` call.
    ///
    /// Each entry represents one player bullet that terminated this tick (wall
    /// hit or out of range), with its muzzle origin and impact position.  The
    /// server calls this after `update()` to build `BulletEvent`s for the
    /// snapshot.
    pub fn take_bullet_traces(&mut self) -> Vec<ImpactEvent> {
        std::mem::take(&mut self.bullet_traces)
    }

    /// Advance the game by one fixed timestep.
    ///
    /// Execution order (all within one tick):
    /// 1. Player movement system
    /// 2. Enemy movement system (AI + physics)
    /// 3. Visibility system (which enemies the player can see)
    /// 4. Projectile system (bullets move and resolve hits)
    /// 5. Impact marks lifetime
    /// 6. Spawn queue flush (with bullet-trace raycasting)
    pub fn update(&mut self, dt: f32, input: &InputState) {
        self.bullet_traces.clear();

        // 1. Player movement
        movement::step_player(
            &mut self.player,
            dt,
            input,
            &self.walls,
            PLAYER_HALF,
            self.level_bounds,
            &mut self.spawn_queue,
        );

        // 2. Enemy movement
        let player_pos = (self.player.movement.x, self.player.movement.y);
        movement::step_enemies(
            &mut self.enemies,
            dt,
            player_pos,
            &self.walls,
            ENEMY_HALF,
            self.level_bounds,
            &mut self.spawn_queue,
        );

        // 3. Visibility
        visibility::sync_enemy_visibility(
            &mut self.enemies,
            player_pos,
            &self.player.sight,
            &self.walls,
        );

        // 4. Projectiles
        let impact_events = projectile::step_projectiles(
            dt,
            &mut self.bullets,
            &mut self.walls,
            &mut self.enemies,
            player_pos,
            ENEMY_HALF,
            PLAYER_HALF,
        );
        self.enemies.retain(|e| e.hp > 0);
        if let Some(bounds) = self.level_bounds {
            self.bullets.retain(|b| {
                b.x >= bounds.x
                    && b.x <= bounds.x + bounds.w
                    && b.y >= bounds.y
                    && b.y <= bounds.y + bounds.h
            });
        }

        // 5. Impact mark lifetimes
        self.impacts.extend(
            impact_events
                .iter()
                .map(|hit| ImpactMark::new(hit.x, hit.y)),
        );
        for impact in &mut self.impacts {
            impact.ttl -= dt;
        }
        self.impacts.retain(|impact| impact.ttl > 0.0);

        // 6. Spawn queue — also raycasts player bullets so the server receives
        //    a `BulletEvent` in the same tick the shot was fired.
        spawn_system::flush_spawn_queue(
            &mut self.spawn_queue,
            &mut self.bullets,
            &mut self.enemies,
            &self.walls,
            &mut self.bullet_traces,
            BULLET_MAX_RANGE,
        );
    }
}
