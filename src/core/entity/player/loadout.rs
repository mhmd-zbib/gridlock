use crate::core::entity::weapon::{WeaponClass, WeaponId, WeaponState, WeaponStats};
use crate::input::InputState;

const PLAYER_LOADOUT: [WeaponId; 6] = [
    WeaponId::Ak47,
    WeaponId::Mp5,
    WeaponId::Sniper,
    WeaponId::M4a1,
    WeaponId::Uzi,
    WeaponId::Dmr,
];

pub struct WeaponLoadout {
    slots: [WeaponState; PLAYER_LOADOUT.len()],
    active_slot: usize,
    was_reload_pressed: bool,
    was_buy_pressed: bool,
    was_digit_pressed: [bool; 5],
    buy_stage: BuyStage,
}

#[derive(Clone, Copy)]
enum BuyStage {
    Closed,
    ClassSelect,
    WeaponSelect(WeaponClass),
}

impl WeaponLoadout {
    pub fn new() -> Self {
        Self {
            slots: PLAYER_LOADOUT.map(WeaponState::new),
            active_slot: 0,
            was_reload_pressed: false,
            was_buy_pressed: false,
            was_digit_pressed: [false; 5],
            buy_stage: BuyStage::Closed,
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
        let buy_pressed = input.key_b;
        if buy_pressed && !self.was_buy_pressed {
            self.buy_stage = match self.buy_stage {
                BuyStage::Closed => BuyStage::ClassSelect,
                BuyStage::ClassSelect | BuyStage::WeaponSelect(_) => BuyStage::Closed,
            };
        }
        self.was_buy_pressed = buy_pressed;

        let digits = [
            input.key_1,
            input.key_2,
            input.key_3,
            input.key_4,
            input.key_5,
        ];
        let mut just_pressed_digit = None;
        for (idx, is_pressed) in digits.iter().enumerate() {
            if *is_pressed && !self.was_digit_pressed[idx] {
                just_pressed_digit = Some(idx + 1);
                break;
            }
        }
        self.was_digit_pressed = digits;

        let Some(choice) = just_pressed_digit else {
            return;
        };

        match self.buy_stage {
            BuyStage::Closed => {}
            BuyStage::ClassSelect => {
                self.buy_stage = match choice {
                    1 => BuyStage::WeaponSelect(WeaponClass::Rifle),
                    2 => BuyStage::WeaponSelect(WeaponClass::Smg),
                    3 => BuyStage::WeaponSelect(WeaponClass::Sniper),
                    _ => BuyStage::ClassSelect,
                };
            }
            BuyStage::WeaponSelect(class) => {
                if let Some(weapon) = weapon_for_class_slot(class, choice) {
                    if let Some(slot_idx) = PLAYER_LOADOUT.iter().position(|id| *id == weapon) {
                        self.active_slot = slot_idx;
                    }
                    self.buy_stage = BuyStage::Closed;
                }
            }
        }
    }

    pub fn buy_prompt(&self) -> Option<&'static str> {
        match self.buy_stage {
            BuyStage::Closed => None,
            BuyStage::ClassSelect => {
                Some("BUY: choose class  1=Rifle  2=SMG  3=Sniper  (B cancel)")
            }
            BuyStage::WeaponSelect(WeaponClass::Rifle) => {
                Some("Rifle: 1=AK-47  2=M4A1  3-5=empty  (B cancel)")
            }
            BuyStage::WeaponSelect(WeaponClass::Smg) => {
                Some("SMG: 1=MP5  2=Uzi  3-5=empty  (B cancel)")
            }
            BuyStage::WeaponSelect(WeaponClass::Sniper) => {
                Some("Sniper: 1=Sniper  2=DMR  3-5=empty  (B cancel)")
            }
        }
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

fn weapon_for_class_slot(class: WeaponClass, slot: usize) -> Option<WeaponId> {
    match (class, slot) {
        (WeaponClass::Rifle, 1) => Some(WeaponId::Ak47),
        (WeaponClass::Rifle, 2) => Some(WeaponId::M4a1),
        (WeaponClass::Smg, 1) => Some(WeaponId::Mp5),
        (WeaponClass::Smg, 2) => Some(WeaponId::Uzi),
        (WeaponClass::Sniper, 1) => Some(WeaponId::Sniper),
        (WeaponClass::Sniper, 2) => Some(WeaponId::Dmr),
        _ => None,
    }
}
