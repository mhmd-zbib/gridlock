use crate::entity::weapon::attachment::{
    AttachmentCategory, AttachmentEffects, AttachmentLoadout, attachment_effects, attachment_name,
};
use crate::entity::weapon::{WeaponId, WeaponState, WeaponStats, all_weapon_ids};

pub struct WeaponLoadout {
    weapon: WeaponState,
    attachments: AttachmentLoadout,
    was_reload_pressed: bool,
}

impl WeaponLoadout {
    pub fn new() -> Self {
        let weapon_ids = all_weapon_ids();
        let Some(first_weapon) = weapon_ids.first().copied() else {
            panic!("weapon catalog is empty");
        };
        Self {
            weapon: WeaponState::new(first_weapon),
            attachments: AttachmentLoadout::default(),
            was_reload_pressed: false,
        }
    }

    pub fn set_weapon(&mut self, weapon_id: WeaponId) {
        self.weapon = WeaponState::new(weapon_id);
        self.weapon.clamp_mag_size(self.active_stats().mag_size);
    }

    pub fn set_attachments(&mut self, attachments: AttachmentLoadout) {
        self.attachments = attachments;
        self.weapon.clamp_mag_size(self.active_stats().mag_size);
    }

    pub fn attachment_name_for(&self, category: AttachmentCategory) -> Option<&'static str> {
        self.attachments.get(category).and_then(attachment_name)
    }

    pub fn weapon_name(&self) -> &'static str {
        self.weapon.stats().name
    }

    pub fn weapon_class_label(&self) -> &'static str {
        self.weapon.stats().class.label()
    }

    pub fn ammo_in_mag(&self) -> u32 {
        self.weapon.ammo_in_mag()
    }

    pub fn mag_size(&self) -> u32 {
        self.active_stats().mag_size
    }

    pub fn is_reloading(&self) -> bool {
        self.weapon.is_reloading()
    }

    pub fn active_stats(&self) -> WeaponStats {
        self.attachment_effects()
            .apply_to_stats(self.weapon.stats())
    }

    pub fn tick(&mut self, dt: f32) {
        let stats = self.active_stats();
        self.weapon.tick_with_stats(stats, dt);
    }

    pub fn update_reload_input(&mut self, reload_pressed: bool) {
        let just_pressed = reload_pressed && !self.was_reload_pressed;
        if just_pressed {
            self.try_start_reload();
        }
        self.was_reload_pressed = reload_pressed;
    }

    pub fn auto_reload_if_dry_trigger(&mut self, trigger_held: bool) {
        if trigger_held && self.ammo_in_mag() == 0 {
            self.try_start_reload();
        }
    }

    pub fn try_fire(&mut self) -> bool {
        let stats = self.active_stats();
        self.weapon.try_fire_with_stats(stats)
    }

    fn try_start_reload(&mut self) -> bool {
        let stats = self.active_stats();
        self.weapon.try_start_reload_with_stats(stats)
    }

    fn attachment_effects(&self) -> AttachmentEffects {
        let mut out = AttachmentEffects::default();
        for category in AttachmentCategory::all() {
            let Some(id) = self.attachments.get(*category) else {
                continue;
            };
            if let Some(fx) = attachment_effects(id) {
                out = out.combine(fx);
            }
        }
        out
    }
}
