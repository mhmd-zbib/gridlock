use crate::bullet::Bullet;
use crate::enemy::Enemy;
use crate::input::InputState;
use crate::level::LevelData;
use crate::player::Player;
use crate::spawn::{SpawnQueue, SpawnRequest};
use crate::wall::{self, Wall};

const PLAYER_HALF: f32 = 10.0;
const ENEMY_HALF:  f32 = 8.0;

pub struct Game {
    pub player:  Player,
    pub enemies: Vec<Enemy>,
    pub bullets: Vec<Bullet>,
    pub walls:   Vec<Wall>,
    spawn_queue: SpawnQueue,
}

impl Game {
    pub fn new() -> Self {
        Self {
            player:      Player::new(400.0, 300.0),
            enemies:     vec![Enemy::new(100.0, 100.0), Enemy::new(700.0, 500.0)],
            bullets:     Vec::new(),
            walls:       Vec::new(),
            spawn_queue: SpawnQueue::default(),
        }
    }

    pub fn load_level(&mut self, level: &LevelData) {
        self.player = match level.player_spawn {
            Some(sp) => Player::new(sp.x, sp.y),
            None     => Player::new(400.0, 300.0),
        };
        self.enemies     = level.enemies.iter().map(|p| Enemy::new(p.x, p.y)).collect();
        self.walls       = level.walls.clone();
        self.bullets     = Vec::new();
        self.spawn_queue = SpawnQueue::default();
        println!("[game] level loaded  enemies={}  walls={}", self.enemies.len(), self.walls.len());
    }

    pub fn update(&mut self, dt: f32, input: &InputState) {
        // --- move entities ---
        self.player.update(dt, input, &mut self.spawn_queue);
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

        for bullet in &mut self.bullets {
            bullet.update(dt);
            if self.walls.iter().any(|w| w.contains(bullet.x, bullet.y)) {
                bullet.alive = false;
            }
        }
        self.bullets.retain(|b| b.alive);

        // --- flush spawn queue ---
        for req in self.spawn_queue.drain() {
            match req {
                SpawnRequest::Bullet { x, y, dir_x, dir_y } => {
                    self.bullets.push(Bullet::new(x, y, dir_x, dir_y));
                }
                SpawnRequest::Enemy { x, y } => {
                    self.enemies.push(Enemy::new(x, y));
                }
            }
        }
    }
}
