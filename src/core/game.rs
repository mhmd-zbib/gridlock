use super::entity::bullet::{Bullet, BulletOwner};
use super::entity::enemy::Enemy;
use super::entity::player::Player;
use super::spawn::{SpawnQueue, SpawnRequest};
use super::world::level::LevelData;
use super::world::rooms::LevelRooms;
use super::world::wall::{self, Wall};
use crate::input::InputState;

const PLAYER_HALF: f32 = 10.0;
const ENEMY_HALF: f32 = 8.0;
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
            player: Player::new(400.0, 300.0),
            enemies: vec![Enemy::new(100.0, 100.0), Enemy::new(700.0, 500.0)],
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
            None => Player::new(400.0, 300.0),
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
        // --- move entities ---
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

        // Compute which enemies are visible to the player.
        for enemy in &mut self.enemies {
            let ep = (enemy.movement.x, enemy.movement.y);
            enemy.visible_to_player = self.player.sight.can_see(target, ep, &self.walls);
        }

        let mut new_impacts = Vec::new();
        for bullet in &mut self.bullets {
            bullet.update(dt);
            if self.walls.iter().any(|w| w.contains(bullet.x, bullet.y)) {
                bullet.alive = false;
                new_impacts.push(ImpactMark::new(bullet.x, bullet.y));
                continue;
            }

            match bullet.owner {
                BulletOwner::Player => {
                    for enemy in &mut self.enemies {
                        let dx = bullet.x - enemy.movement.x;
                        let dy = bullet.y - enemy.movement.y;
                        if (dx * dx + dy * dy).sqrt() < ENEMY_HALF * 2.0 {
                            bullet.alive = false;
                            enemy.hp = enemy.hp.saturating_sub(bullet.damage);
                            new_impacts.push(ImpactMark::new(bullet.x, bullet.y));
                            break;
                        }
                    }
                }
                BulletOwner::Enemy => {
                    let dx = bullet.x - self.player.movement.x;
                    let dy = bullet.y - self.player.movement.y;
                    if (dx * dx + dy * dy).sqrt() < PLAYER_HALF * 2.0 {
                        bullet.alive = false;
                        new_impacts.push(ImpactMark::new(bullet.x, bullet.y));
                    }
                }
            }
        }
        self.bullets.retain(|b| b.alive);
        self.enemies.retain(|e| e.hp > 0);
        self.impacts.extend(new_impacts);
        for impact in &mut self.impacts {
            impact.ttl -= dt;
        }
        self.impacts.retain(|impact| impact.ttl > 0.0);

        // --- flush spawn queue ---
        for req in self.spawn_queue.drain() {
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
                    self.bullets
                        .push(Bullet::new(x, y, dir_x, dir_y, speed, damage, owner));
                }
                SpawnRequest::Enemy { x, y } => {
                    self.enemies.push(Enemy::new(x, y));
                }
            }
        }
    }
}
