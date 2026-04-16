mod loadout;
mod locomotion;

use super::bullet::BulletOwner;
use crate::entity::movement::Movement;
use crate::entity::weapon::attachment::{AttachmentCategory, AttachmentLoadout};
use crate::entity::weapon::{
    WeaponId, all_weapon_ids, weapon_supports_attachment, weapon_supports_attachment_category,
};
use crate::input::InputState;
use crate::spawn::{SpawnQueue, SpawnRequest};
use crate::world::aim_cone::AimCone;
use crate::world::sight::Sight;
use crate::world::units::px_to_tiles;
use crate::world::wall::Wall;

use self::loadout::WeaponLoadout;
use self::locomotion::{LocomotionState, RUN_SPEED};

pub struct Player {
    pub movement: Movement,
    pub sight: Sight,
    pub aim_cone: AimCone,
    locomotion: LocomotionState,
    loadout: WeaponLoadout,
}

#[derive(Clone)]
pub struct PlayerLoadoutConfig {
    pub weapon: WeaponId,
    pub attachments: AttachmentLoadout,
}

impl Default for PlayerLoadoutConfig {
    fn default() -> Self {
        let all = all_weapon_ids();
        let Some(first_weapon) = all.first().copied() else {
            panic!("weapon catalog is empty");
        };
        Self {
            weapon: first_weapon,
            attachments: AttachmentLoadout::default(),
        }
    }
}

/// Remove any attachment that is incompatible with the chosen weapon.
///
/// Called after the player edits their loadout config so the game state never
/// holds an attachment that the weapon does not support.
pub fn sanitize_loadout(loadout: &mut PlayerLoadoutConfig) {
    for category in AttachmentCategory::all() {
        if weapon_supports_attachment_category(loadout.weapon, *category) {
            if let Some(attachment) = loadout.attachments.get(*category) {
                if !weapon_supports_attachment(loadout.weapon, attachment) {
                    loadout.attachments.unequip(*category);
                }
            }
        } else {
            loadout.attachments.unequip(*category);
        }
    }
}

impl Player {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            movement: Movement::new(x, y, px_to_tiles(100.0)),
            sight: Sight::player(),
            aim_cone: AimCone::new(),
            locomotion: LocomotionState::new(),
            loadout: WeaponLoadout::new(),
        }
    }

    pub fn weapon_name(&self) -> &'static str {
        self.loadout.weapon_name()
    }

    pub fn weapon_class_label(&self) -> &'static str {
        self.loadout.weapon_class_label()
    }

    pub fn ammo_in_mag(&self) -> u32 {
        self.loadout.ammo_in_mag()
    }

    pub fn mag_size(&self) -> u32 {
        self.loadout.mag_size()
    }

    pub fn is_reloading(&self) -> bool {
        self.loadout.is_reloading()
    }

    pub fn attachment_name_for(&self, category: AttachmentCategory) -> Option<&'static str> {
        self.loadout.attachment_name_for(category)
    }

    pub fn apply_loadout(&mut self, config: &PlayerLoadoutConfig) {
        self.loadout.set_weapon(config.weapon);
        self.loadout.set_attachments(config.attachments.clone());
    }

    pub fn update(
        &mut self,
        dt: f32,
        input: &InputState,
        walls: &[Wall],
        half_size: f32,
        spawns: &mut SpawnQueue,
    ) {
        self.locomotion
            .step(dt, input, &mut self.movement, half_size, walls);

        let from = (self.movement.x, self.movement.y);
        let to = (
            px_to_tiles(input.mouse_x as f32),
            px_to_tiles(input.mouse_y as f32),
        );
        self.sight.face(from, to, dt);
        self.aim_cone.direction = self.sight.direction;

        self.loadout.tick(dt);
        self.loadout.update_reload_input(input.reload);
        self.loadout.auto_reload_if_dry_trigger(input.shoot);

        let active_stats = self.loadout.active_stats();
        self.sight.range = active_stats.visibility_range;
        self.sight.half_angle = active_stats.visibility_half_angle_deg.to_radians();
        self.aim_cone.set_spread_profile(
            active_stats.aim_base_half_angle_deg,
            active_stats.movement_spread_max_deg,
            active_stats.aim_cone_render_range,
        );

        let speed_frac = self.movement.velocity_frac * (self.movement.speed / RUN_SPEED);
        self.aim_cone
            .update(dt, speed_frac, active_stats.recoil_decay_deg_per_sec);

        if input.shoot && self.loadout.try_fire() {
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
}
