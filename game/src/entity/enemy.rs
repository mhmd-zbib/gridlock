use super::bullet::BulletOwner;
use super::movement::Movement;
use crate::ai::brain::EnemyBrain;
use crate::spawn::{SpawnQueue, SpawnRequest};
use crate::world::sight::Sight;
use crate::world::units::px_to_tiles;
use crate::world::wall::{self, Wall};

const MAX_HP: u32 = 3;
const ENEMY_BULLET_SPEED: f32 = px_to_tiles(600.0);
const ENEMY_BULLET_DAMAGE: u32 = 1;
/// Must match ENEMY_HALF in game.rs — the collision half-extent used for push_out.
const ENEMY_HALF: f32 = px_to_tiles(8.0);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EnemyKind {
    Shooter,
    TargetDummy,
}

/// The physical enemy entity: position, sight, health.
///
/// All cognition (perception, suspicion, behaviour) lives in `EnemyBrain`.
/// This struct owns the body; the brain owns the mind.
pub struct Enemy {
    pub kind: EnemyKind,
    pub movement: Movement,
    pub sight: Sight,
    /// Set by `game.update()` — used by the renderer to cull invisible enemies.
    pub visible_to_player: bool,
    /// All AI state. Exposed so the renderer can read suspicion for cone colour.
    pub brain: EnemyBrain,
    pub hp: u32,
}

impl Enemy {
    pub fn new(x: f32, y: f32) -> Self {
        Self::new_with_kind(x, y, EnemyKind::Shooter)
    }

    pub fn target_dummy(x: f32, y: f32) -> Self {
        Self::new_with_kind(x, y, EnemyKind::TargetDummy)
    }

    fn new_with_kind(x: f32, y: f32, kind: EnemyKind) -> Self {
        let sight = Sight::enemy();
        let brain = EnemyBrain::new((x, y), sight.direction);
        Self {
            kind,
            movement: Movement::new(x, y, px_to_tiles(90.0)),
            sight,
            visible_to_player: true,
            brain,
            hp: MAX_HP,
        }
    }

    pub fn update(
        &mut self,
        dt: f32,
        player_pos: (f32, f32),
        walls: &[Wall],
        spawns: &mut SpawnQueue,
    ) {
        if self.kind == EnemyKind::TargetDummy {
            self.movement.velocity_frac = 0.0;
            return;
        }

        let from = (self.movement.x, self.movement.y);

        // The brain reads sight.direction for smoothed rotation and perception.
        // It returns instructions; this function applies them.
        let output = self.brain.update(from, &self.sight, player_pos, walls, dt);

        self.sight.direction = output.look_dir;

        if let Some(target) = output.move_target {
            self.move_toward(target, walls, dt);
        }

        if let Some((dir_x, dir_y)) = output.shoot {
            spawns.push(SpawnRequest::Bullet {
                x: from.0,
                y: from.1,
                dir_x,
                dir_y,
                speed: ENEMY_BULLET_SPEED,
                damage: ENEMY_BULLET_DAMAGE,
                owner: BulletOwner::Enemy,
            });
        }
    }

    /// Move toward `target` at movement speed, sliding along walls rather than
    /// jittering into them.
    ///
    /// Strategy:
    ///   1. Compute a normalized step vector (capped so we don't overshoot).
    ///   2. Try the full diagonal move and resolve collisions.
    ///   3. If the full move was mostly blocked, try sliding on each axis
    ///      independently and pick whichever makes more progress.
    ///   4. If all axes are blocked, stop cleanly.
    fn move_toward(&mut self, target: (f32, f32), walls: &[Wall], dt: f32) {
        let fx = self.movement.x;
        let fy = self.movement.y;
        let dx = target.0 - fx;
        let dy = target.1 - fy;
        let dist = dx.hypot(dy);
        if dist < px_to_tiles(1.0) {
            self.movement.velocity_frac = 0.0;
            return;
        }

        // Normalised step — never overshoot the target.
        let step = (self.movement.speed * dt).min(dist);
        let nx = dx / dist * step;
        let ny = dy / dist * step;

        // ── 1. Try the full diagonal move ────────────────────────────────────
        let (mut cx, mut cy) = (fx + nx, fy + ny);
        wall::resolve_all(&mut cx, &mut cy, ENEMY_HALF, walls);
        let full_progress = (cx - fx).hypot(cy - fy);

        if full_progress >= step * 0.5 {
            // Wall only clipped the corner; accept this result.
            self.movement.x = cx;
            self.movement.y = cy;
            self.movement.velocity_frac = (full_progress / step).min(1.0);
            return;
        }

        // ── 2. Slide along X only ─────────────────────────────────────────────
        let (mut sx, mut sy) = (fx + nx, fy);
        wall::resolve_all(&mut sx, &mut sy, ENEMY_HALF, walls);
        let x_progress = (sx - fx).abs();

        // ── 3. Slide along Y only ─────────────────────────────────────────────
        let (mut qx, mut qy) = (fx, fy + ny);
        wall::resolve_all(&mut qx, &mut qy, ENEMY_HALF, walls);
        let y_progress = (qy - fy).abs();

        if x_progress >= y_progress && x_progress > px_to_tiles(0.5) {
            self.movement.x = sx;
            self.movement.y = sy;
            self.movement.velocity_frac = (x_progress / step).min(1.0);
        } else if y_progress > px_to_tiles(0.5) {
            self.movement.x = qx;
            self.movement.y = qy;
            self.movement.velocity_frac = (y_progress / step).min(1.0);
        } else {
            // Fully blocked — stop without jitter.
            self.movement.velocity_frac = 0.0;
        }
    }
}
