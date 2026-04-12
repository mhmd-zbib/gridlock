use super::movement::{Movement, MovementInput};
use crate::core::world::sight::Sight;
use crate::core::spawn::{SpawnQueue, SpawnRequest};
use crate::core::world::wall::Wall;

// How long an enemy keeps chasing after losing sight of the player.
const ALERT_DURATION: f32 = 2.5;
// How long between enemy shots (seconds).
const SHOOT_INTERVAL: f32 = 2.2;
// Patrol cone sweep amplitude (radians).
const SWEEP_AMP: f32 = 0.75;

pub struct Enemy {
    pub movement:          Movement,
    pub sight:             Sight,
    /// Set by `game.update()` — used by the renderer to cull invisible enemies.
    pub visible_to_player: bool,
    /// Whether this enemy can currently see the player.
    pub sees_player:       bool,

    // AI state
    pub alert_timer:  f32,  // > 0 while alerted after losing sight
    shoot_timer:  f32,  // counts down to next shot
    sweep_timer:  f32,  // drives patrol sweep oscillation
    base_dir:     f32,  // patrol base direction (set at spawn)
}

impl Enemy {
    pub fn is_alerted(&self) -> bool { self.alert_timer > 0.0 }

    pub fn new(x: f32, y: f32) -> Self {
        Self {
            movement:          Movement::new(x, y, 90.0),
            sight:             Sight::enemy(),
            visible_to_player: true, // default visible until game computes it
            sees_player:       false,
            alert_timer:       0.0,
            shoot_timer:       SHOOT_INTERVAL,
            sweep_timer:       0.0,
            base_dir:          0.0,
        }
    }

    pub fn update(
        &mut self,
        dt:         f32,
        player_pos: (f32, f32),
        walls:      &[Wall],
        spawns:     &mut SpawnQueue,
    ) {
        let from = (self.movement.x, self.movement.y);

        // --- sight check ---
        self.sees_player = self.sight.can_see(from, player_pos, walls);

        if self.sees_player {
            // ---- CHASE ----
            self.alert_timer = ALERT_DURATION;

            // Face and move toward player.
            self.sight.face(from, player_pos, dt);
            self.base_dir = self.sight.direction;

            let dx = player_pos.0 - from.0;
            let dy = player_pos.1 - from.1;
            self.movement.apply(MovementInput {
                up:    dy < 0.0,
                down:  dy > 0.0,
                left:  dx < 0.0,
                right: dx > 0.0,
            }, dt);

            // Shoot at player.
            self.shoot_timer -= dt;
            if self.shoot_timer <= 0.0 {
                self.shoot_timer = SHOOT_INTERVAL;
                let len = (dx * dx + dy * dy).sqrt();
                if len > 0.0 {
                    spawns.push(SpawnRequest::Bullet {
                        x: from.0, y: from.1,
                        dir_x: dx / len,
                        dir_y: dy / len,
                    });
                }
            }
        } else if self.alert_timer > 0.0 {
            // ---- ALERTED — lost sight, searching ----
            self.alert_timer -= dt;
            // Sweep quickly left and right.
            self.sweep_timer += dt;
            self.sight.direction = self.base_dir
                + (self.sweep_timer * 2.0).sin() * SWEEP_AMP * 1.5;
            // Stand still while searching.
        } else {
            // ---- PATROL — slowly sweep cone ----
            self.sweep_timer += dt;
            self.sight.direction = self.base_dir
                + (self.sweep_timer * 0.45).sin() * SWEEP_AMP;
            // No movement during patrol.
        }
    }
}
