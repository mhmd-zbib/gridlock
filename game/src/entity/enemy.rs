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

    fn move_toward(&mut self, target: (f32, f32), walls: &[Wall], dt: f32) {
        let from = (self.movement.x, self.movement.y);
        let Some((nx, ny, step)) = step_toward(from, target, self.movement.speed * dt) else {
            self.movement.velocity_frac = 0.0;
            return;
        };
        let (new_pos, vel_frac) = apply_move_with_slide(from, nx, ny, step, ENEMY_HALF, walls);
        self.movement.x = new_pos.0;
        self.movement.y = new_pos.1;
        self.movement.velocity_frac = vel_frac;
    }
}

// ---------------------------------------------------------------------------
// Movement helpers
// ---------------------------------------------------------------------------

/// Returns a step vector `(nx, ny)` and its magnitude `step`, capped so the
/// entity never overshoots `target`. Returns `None` if already within 1px.
fn step_toward(
    from: (f32, f32),
    target: (f32, f32),
    max_step: f32,
) -> Option<(f32, f32, f32)> {
    let dx = target.0 - from.0;
    let dy = target.1 - from.1;
    let dist = dx.hypot(dy);
    if dist < px_to_tiles(1.0) {
        return None;
    }
    let step = max_step.min(dist);
    Some((dx / dist * step, dy / dist * step, step))
}

/// Try a diagonal move; if mostly blocked, slide along the better axis.
/// Returns the resolved position and the velocity fraction [0, 1].
fn apply_move_with_slide(
    from: (f32, f32),
    nx: f32,
    ny: f32,
    step: f32,
    half: f32,
    walls: &[Wall],
) -> ((f32, f32), f32) {
    let (mut cx, mut cy) = (from.0 + nx, from.1 + ny);
    wall::resolve_all(&mut cx, &mut cy, half, walls);
    let diag_progress = (cx - from.0).hypot(cy - from.1);
    if diag_progress >= step * 0.5 {
        return ((cx, cy), (diag_progress / step).min(1.0));
    }

    let (mut sx, mut sy) = (from.0 + nx, from.1);
    wall::resolve_all(&mut sx, &mut sy, half, walls);
    let x_progress = (sx - from.0).abs();

    let (mut qx, mut qy) = (from.0, from.1 + ny);
    wall::resolve_all(&mut qx, &mut qy, half, walls);
    let y_progress = (qy - from.1).abs();

    if x_progress >= y_progress && x_progress > px_to_tiles(0.5) {
        ((sx, sy), (x_progress / step).min(1.0))
    } else if y_progress > px_to_tiles(0.5) {
        ((qx, qy), (y_progress / step).min(1.0))
    } else {
        (from, 0.0)
    }
}
