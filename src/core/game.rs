use super::entity::bullet::Bullet;
use super::entity::enemy::Enemy;
use super::entity::player::{Player, PlayerLoadoutConfig};
use super::entity::weapon::attachment::AttachmentCategory;
use super::entity::weapon::{weapon_supports_attachment, weapon_supports_attachment_category};
use super::spawn::SpawnQueue;
use super::systems::{projectile, spawn, visibility};
use super::world::level::{LevelBounds, LevelData};
use super::world::prop::{self, ResolvedProp};
use super::world::rooms::LevelRooms;
use super::world::units::px_to_tiles;
use super::world::wall::{self, Wall};
use crate::input::InputState;

const PLAYER_HALF: f32 = px_to_tiles(10.0);
const ENEMY_HALF: f32 = px_to_tiles(8.0);
const IMPACT_TTL: f32 = 0.15;

pub struct ImpactMark {
    pub x: f32,
    pub y: f32,
    ttl: f32,
}

impl ImpactMark {
    fn new(x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            ttl: IMPACT_TTL,
        }
    }

    pub fn alpha(&self) -> f32 {
        (self.ttl / IMPACT_TTL).clamp(0.0, 1.0)
    }
}

pub struct Game {
    pub player: Player,
    pub enemies: Vec<Enemy>,
    pub bullets: Vec<Bullet>,
    pub impacts: Vec<ImpactMark>,
    pub walls: Vec<Wall>,
    pub props: Vec<ResolvedProp>,
    pub rooms: LevelRooms,
    pub level_bounds: Option<LevelBounds>,
    spawn_queue: SpawnQueue,
    player_loadout: PlayerLoadoutConfig,
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
            props: Vec::new(),
            rooms: LevelRooms::default(),
            level_bounds: None,
            spawn_queue: SpawnQueue::default(),
            player_loadout,
        }
    }

    pub fn set_player_loadout(&mut self, loadout: PlayerLoadoutConfig) {
        let mut sanitized = loadout;
        sanitize_loadout_for_weapon(&mut sanitized);
        self.player_loadout = sanitized;
        self.player.apply_loadout(&self.player_loadout);
    }

    pub fn player_loadout(&self) -> PlayerLoadoutConfig {
        self.player_loadout.clone()
    }

    pub fn load_level(&mut self, level: &LevelData, level_width: f32, level_height: f32) {
        self.player = match level.player_spawn {
            Some(sp) => Player::new(sp.x, sp.y),
            None => Player::new(px_to_tiles(400.0), px_to_tiles(300.0)),
        };
        sanitize_loadout_for_weapon(&mut self.player_loadout);
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

        let prop_assets = prop::load_assets();
        self.props = prop::resolve_level_props(&level.props, &prop_assets);
        self.walls = level.walls.clone();
        for wall in &mut self.walls {
            wall.init_segments();
        }
        let collider_props = self.props.iter().filter(|p| p.is_collider).count();
        for prop in self.props.iter().filter(|p| p.is_collider) {
            self.walls.push(Wall::new(
                prop.x - prop.width * 0.5,
                prop.y - prop.height * 0.5,
                prop.width,
                prop.height,
            ));
        }
        self.bullets = Vec::new();
        self.impacts = Vec::new();
        self.spawn_queue = SpawnQueue::default();
        self.level_bounds = level.map_bounds;
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

        let room_w = match level.map_bounds {
            Some(bounds) => (bounds.x + bounds.w).max(level_width),
            None => level_width,
        };
        let room_h = match level.map_bounds {
            Some(bounds) => (bounds.y + bounds.h).max(level_height),
            None => level_height,
        };

        // Compute rooms and gaps ONCE at level load (cached for debug rendering)
        self.rooms = LevelRooms::compute(&self.walls, room_w, room_h);

        println!(
            "[game] level loaded  enemies={}  targets={}  walls={}  props={}  collider_props={}  rooms={}  gaps={}  outside_cells={}",
            self.enemies.len(),
            level.target_enemies.len(),
            self.walls.len(),
            self.props.len(),
            collider_props,
            self.rooms.rooms.len(),
            self.rooms.gaps.len(),
            self.rooms.outside_cells
        );
    }

    pub fn update(&mut self, dt: f32, input: &InputState) {
        // --- entity updates ---
        self.player
            .update(dt, input, &self.walls, PLAYER_HALF, &mut self.spawn_queue);
        wall::resolve_all(
            &mut self.player.movement.x,
            &mut self.player.movement.y,
            PLAYER_HALF,
            &self.walls,
        );
        clamp_actor_to_level_bounds(
            &mut self.player.movement.x,
            &mut self.player.movement.y,
            PLAYER_HALF,
            self.level_bounds,
        );

        let target = (self.player.movement.x, self.player.movement.y);
        for enemy in &mut self.enemies {
            enemy.update(dt, target, &self.walls, &mut self.spawn_queue);
            wall::resolve_all(
                &mut enemy.movement.x,
                &mut enemy.movement.y,
                ENEMY_HALF,
                &self.walls,
            );
            clamp_actor_to_level_bounds(
                &mut enemy.movement.x,
                &mut enemy.movement.y,
                ENEMY_HALF,
                self.level_bounds,
            );
        }

        visibility::sync_enemy_visibility(
            &mut self.enemies,
            target,
            &self.player.sight,
            &self.walls,
        );

        let impact_events = projectile::step_projectiles(
            dt,
            &mut self.bullets,
            &mut self.walls,
            &mut self.enemies,
            target,
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
        self.impacts.extend(
            impact_events
                .into_iter()
                .map(|hit| ImpactMark::new(hit.x, hit.y)),
        );
        for impact in &mut self.impacts {
            impact.ttl -= dt;
        }
        self.impacts.retain(|impact| impact.ttl > 0.0);

        spawn::flush_spawn_queue(&mut self.spawn_queue, &mut self.bullets, &mut self.enemies);
    }
}

fn clamp_actor_to_level_bounds(x: &mut f32, y: &mut f32, half: f32, bounds: Option<LevelBounds>) {
    let Some(bounds) = bounds else {
        return;
    };
    let min_x = bounds.x + half;
    let max_x = bounds.x + bounds.w - half;
    let min_y = bounds.y + half;
    let max_y = bounds.y + bounds.h - half;

    if min_x <= max_x {
        *x = x.clamp(min_x, max_x);
    } else {
        *x = bounds.x + bounds.w * 0.5;
    }
    if min_y <= max_y {
        *y = y.clamp(min_y, max_y);
    } else {
        *y = bounds.y + bounds.h * 0.5;
    }
}

fn sanitize_loadout_for_weapon(loadout: &mut PlayerLoadoutConfig) {
    for category in AttachmentCategory::all() {
        if weapon_supports_attachment_category(loadout.weapon, *category) {
            if let Some(attachment) = loadout.attachments.get(*category)
                && !weapon_supports_attachment(loadout.weapon, attachment)
            {
                loadout.attachments.unequip(*category);
            }
        } else {
            loadout.attachments.unequip(*category);
        }
    }
}
