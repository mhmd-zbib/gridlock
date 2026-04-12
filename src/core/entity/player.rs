use super::bullet::BulletOwner;
use super::movement::{Movement, MovementInput};
use super::weapon::{WeaponId, WeaponState};
use crate::core::spawn::{SpawnQueue, SpawnRequest};
use crate::core::world::aim_cone::AimCone;
use crate::core::world::sight::Sight;
use crate::core::world::wall::Wall;
use crate::input::InputState;

const WALK_SPEED: f32 = 40.0;
const NORMAL_SPEED: f32 = 100.0;
const RUN_SPEED: f32 = 280.0;
const SPRINT_BURST_SECS: f32 = 1.2;
const SPRINT_COOLDOWN_SECS: f32 = 8.0;
const PEEK_DISTANCE: f32 = 18.0;

pub struct Player {
    pub movement: Movement,
    pub sight: Sight,
    pub aim_cone: AimCone,
    rifle: WeaponState,
    smg: WeaponState,
    sniper: WeaponState,
    active_weapon: WeaponId,
    was_reload_pressed: bool,
    was_ak_select_pressed: bool,
    was_mp5_select_pressed: bool,
    was_sniper_select_pressed: bool,
    peek_origin: Option<(f32, f32)>,
    was_peeking: bool,
    sprint_time_left: f32,
    sprint_cooldown_left: f32,
}

impl Player {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            movement: Movement::new(x, y, NORMAL_SPEED),
            sight: Sight::player(),
            aim_cone: AimCone::new(),
            rifle: WeaponState::new(WeaponId::Ak47),
            smg: WeaponState::new(WeaponId::Mp5),
            sniper: WeaponState::new(WeaponId::Sniper),
            active_weapon: WeaponId::Ak47,
            was_reload_pressed: false,
            was_ak_select_pressed: false,
            was_mp5_select_pressed: false,
            was_sniper_select_pressed: false,
            peek_origin: None,
            was_peeking: false,
            sprint_time_left: SPRINT_BURST_SECS,
            sprint_cooldown_left: 0.0,
        }
    }

    pub fn weapon_name(&self) -> &'static str {
        self.active_weapon_state().stats().name
    }

    pub fn weapon_class_label(&self) -> &'static str {
        self.active_weapon_state().stats().class.label()
    }

    pub fn ammo_in_mag(&self) -> u32 {
        self.active_weapon_state().ammo_in_mag()
    }

    pub fn mag_size(&self) -> u32 {
        self.active_weapon_state().stats().mag_size
    }

    pub fn is_reloading(&self) -> bool {
        self.active_weapon_state().is_reloading()
    }

    pub fn update(
        &mut self,
        dt: f32,
        input: &InputState,
        walls: &[Wall],
        half_size: f32,
        spawns: &mut SpawnQueue,
    ) {
        if self.sprint_cooldown_left > 0.0 {
            self.sprint_cooldown_left = (self.sprint_cooldown_left - dt).max(0.0);
            if self.sprint_cooldown_left <= 0.0 {
                self.sprint_time_left = SPRINT_BURST_SECS;
            }
        }

        let sprinting =
            input.shift && self.sprint_cooldown_left <= 0.0 && self.sprint_time_left > 0.0;
        if sprinting {
            self.sprint_time_left = (self.sprint_time_left - dt).max(0.0);
            if self.sprint_time_left <= 0.0 {
                self.sprint_cooldown_left = SPRINT_COOLDOWN_SECS;
            }
        }

        self.movement.speed = if sprinting {
            RUN_SPEED
        } else if input.walk {
            WALK_SPEED
        } else {
            NORMAL_SPEED
        };

        let px: f32 = if input.right {
            1.0
        } else if input.left {
            -1.0
        } else {
            0.0
        };
        let py: f32 = if input.down {
            1.0
        } else if input.up {
            -1.0
        } else {
            0.0
        };
        let has_dir = px != 0.0 || py != 0.0;
        if input.peek && self.peek_origin.is_none() {
            self.peek_origin = Some((self.movement.x, self.movement.y));
        }
        let is_peeking = input.peek && has_dir;

        if is_peeking {
            let origin = self
                .peek_origin
                .unwrap_or((self.movement.x, self.movement.y));
            let len = (px * px + py * py).sqrt();
            let dir = (px / len, py / len);
            let dist = clamped_peek_distance(origin, dir, PEEK_DISTANCE, half_size, walls);
            self.movement.x = origin.0 + dir.0 * dist;
            self.movement.y = origin.1 + dir.1 * dist;
            self.movement.velocity_frac = 0.0;
            self.was_peeking = true;
        } else {
            if self.was_peeking && !has_dir {
                if let Some(origin) = self.peek_origin {
                    self.movement.x = origin.0;
                    self.movement.y = origin.1;
                }
            }
            self.was_peeking = false;

            if !input.peek {
                self.peek_origin = None;
                let mv = MovementInput {
                    up: input.up,
                    down: input.down,
                    left: input.left,
                    right: input.right,
                };
                self.movement.apply(mv, dt);
            } else {
                self.movement.velocity_frac = 0.0;
            }
        }

        let from = (self.movement.x, self.movement.y);
        let to = (input.mouse_x as f32, input.mouse_y as f32);

        // Sight and aim cone share the same facing direction.
        self.sight.face(from, to, dt);
        self.aim_cone.direction = self.sight.direction;

        self.rifle.tick(dt);
        self.smg.tick(dt);
        self.sniper.tick(dt);

        if input.key_5 && !self.was_ak_select_pressed {
            self.active_weapon = WeaponId::Ak47;
        }
        if input.key_6 && !self.was_mp5_select_pressed {
            self.active_weapon = WeaponId::Mp5;
        }
        if input.key_7 && !self.was_sniper_select_pressed {
            self.active_weapon = WeaponId::Sniper;
        }
        self.was_ak_select_pressed = input.key_5;
        self.was_mp5_select_pressed = input.key_6;
        self.was_sniper_select_pressed = input.key_7;

        let reload_pressed = input.reload && !self.was_reload_pressed;
        if reload_pressed {
            self.active_weapon_state_mut().try_start_reload();
        }
        self.was_reload_pressed = input.reload;

        {
            let active = self.active_weapon_state_mut();
            if input.shoot && active.ammo_in_mag() == 0 {
                active.try_start_reload();
            }
        }

        let active_stats = self.active_weapon_state().stats();
        self.sight.range = active_stats.visibility_range;
        self.sight.half_angle = active_stats.visibility_half_angle_deg.to_radians();
        self.aim_cone.set_spread_profile(
            active_stats.aim_base_half_angle_deg,
            active_stats.movement_spread_max_deg,
            active_stats.aim_cone_render_range,
        );
        // Scale velocity fraction by speed ratio so walk = tight cone, run = wide cone.
        let speed_frac = self.movement.velocity_frac * (self.movement.speed / RUN_SPEED);
        self.aim_cone
            .update(dt, speed_frac, active_stats.recoil_decay_deg_per_sec);

        let fired = if input.shoot {
            self.active_weapon_state_mut().try_fire()
        } else {
            false
        };
        if fired {
            let (dir_x, dir_y) = self.aim_cone.sample_direction();
            spawns.push(SpawnRequest::Bullet {
                x: from.0,
                y: from.1,
                dir_x,
                dir_y,
                speed: active_stats.bullet_speed,
                damage: active_stats.bullet_damage,
                owner: BulletOwner::Player,
            });
            self.aim_cone.on_shot(
                active_stats.recoil_per_shot_deg,
                active_stats.recoil_max_deg,
            );
        }
    }

    fn active_weapon_state(&self) -> &WeaponState {
        match self.active_weapon {
            WeaponId::Ak47 => &self.rifle,
            WeaponId::Mp5 => &self.smg,
            WeaponId::Sniper => &self.sniper,
        }
    }

    fn active_weapon_state_mut(&mut self) -> &mut WeaponState {
        match self.active_weapon {
            WeaponId::Ak47 => &mut self.rifle,
            WeaponId::Mp5 => &mut self.smg,
            WeaponId::Sniper => &mut self.sniper,
        }
    }
}

fn clamped_peek_distance(
    origin: (f32, f32),
    dir: (f32, f32),
    max_dist: f32,
    half_size: f32,
    walls: &[Wall],
) -> f32 {
    // Step forward in small increments so corner/diagonal peeks can't tunnel through walls.
    const PEEK_STEP: f32 = 0.5;
    let mut safe_dist = 0.0;
    let mut d = 0.0;
    while d < max_dist {
        d = (d + PEEK_STEP).min(max_dist);
        let cx = origin.0 + dir.0 * d;
        let cy = origin.1 + dir.1 * d;
        if walls.iter().any(|w| w.overlaps(cx, cy, half_size)) {
            break;
        }
        safe_dist = d;
    }
    safe_dist
}
