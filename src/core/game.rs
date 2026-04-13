use super::entity::bullet::Bullet;
use super::entity::enemy::Enemy;
use super::entity::player::Player;
use super::spawn::SpawnQueue;
use super::systems::{projectile, spawn, visibility};
use super::world::level::LevelData;
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
    pub rooms: LevelRooms,
    spawn_queue: SpawnQueue,
}

impl Game {
    pub fn new() -> Self {
        Self {
            player: Player::new(px_to_tiles(400.0), px_to_tiles(300.0)),
            enemies: vec![
                Enemy::new(px_to_tiles(100.0), px_to_tiles(100.0)),
                Enemy::new(px_to_tiles(700.0), px_to_tiles(500.0)),
            ],
            bullets: Vec::new(),
            impacts: Vec::new(),
            walls: Vec::new(),
            rooms: LevelRooms::default(),
            spawn_queue: SpawnQueue::default(),
        }
    }

    pub fn load_level(&mut self, level: &LevelData, level_width: f32, level_height: f32) {
        self.player = match level.player_spawn {
            Some(sp) => Player::new(sp.x, sp.y),
            None => Player::new(px_to_tiles(400.0), px_to_tiles(300.0)),
        };
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
        self.walls = level.walls.clone();
        self.bullets = Vec::new();
        self.impacts = Vec::new();
        self.spawn_queue = SpawnQueue::default();

        // Compute rooms and gaps ONCE at level load (cached for debug rendering)
        self.rooms = LevelRooms::compute(&self.walls, level_width, level_height);

        println!(
            "[game] level loaded  enemies={}  targets={}  walls={}  rooms={}  gaps={}  outside_cells={}",
            self.enemies.len(),
            level.target_enemies.len(),
            self.walls.len(),
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

        let target = (self.player.movement.x, self.player.movement.y);
        for enemy in &mut self.enemies {
            enemy.update(dt, target, &self.walls, &mut self.spawn_queue);
            wall::resolve_all(
                &mut enemy.movement.x,
                &mut enemy.movement.y,
                ENEMY_HALF,
                &self.walls,
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
