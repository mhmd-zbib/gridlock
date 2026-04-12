use crate::core::entity::weapon::{WeaponId, WeaponState, WeaponStats};
use crate::input::InputState;

const PLAYER_LOADOUT: [WeaponId; 3] = [WeaponId::Ak47, WeaponId::Mp5, WeaponId::Sniper];

pub struct WeaponLoadout {
    slots: [WeaponState; PLAYER_LOADOUT.len()],
    active_slot: usize,
    was_reload_pressed: bool,
    was_select_pressed: [bool; PLAYER_LOADOUT.len()],
}

impl WeaponLoadout {
    pub fn new() -> Self {
        Self {
            slots: PLAYER_LOADOUT.map(WeaponState::new),
            active_slot: 0,
            was_reload_pressed: false,
            was_select_pressed: [false; PLAYER_LOADOUT.len()],
        }
    }

    pub fn weapon_name(&self) -> &'static str {
        self.active_state().stats().name
    }

    pub fn weapon_class_label(&self) -> &'static str {
        self.active_state().stats().class.label()
    }

    pub fn ammo_in_mag(&self) -> u32 {
        self.active_state().ammo_in_mag()
    }

    pub fn mag_size(&self) -> u32 {
        self.active_state().stats().mag_size
    }

    pub fn is_reloading(&self) -> bool {
        self.active_state().is_reloading()
    }

    pub fn active_stats(&self) -> WeaponStats {
        self.active_state().stats()
    }

    pub fn tick(&mut self, dt: f32) {
        for slot in &mut self.slots {
            slot.tick(dt);
        }
    }

    pub fn update_selection(&mut self, input: &InputState) {
        let pressed = [input.key_5, input.key_6, input.key_7];
        for (idx, is_pressed) in pressed.iter().enumerate() {
            if *is_pressed && !self.was_select_pressed[idx] {
                self.active_slot = idx;
            }
        }
        self.was_select_pressed = pressed;
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
        self.active_state_mut().try_fire()
    }

    fn try_start_reload(&mut self) -> bool {
        self.active_state_mut().try_start_reload()
    }

    fn active_state(&self) -> &WeaponState {
        &self.slots[self.active_slot]
    }

    fn active_state_mut(&mut self) -> &mut WeaponState {
        &mut self.slots[self.active_slot]
    }
}
