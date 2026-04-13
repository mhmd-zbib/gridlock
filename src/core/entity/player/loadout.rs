use crate::core::entity::weapon::{
    WeaponClass, WeaponState, WeaponStats, all_weapon_ids, weapon_classes, weapon_ids_for_class,
};
use crate::input::InputState;

const WEAPON_PAGE_SIZE: usize = 3;

pub struct WeaponLoadout {
    slots: Vec<WeaponState>,
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
    WeaponSelect { class: WeaponClass, page: usize },
}

impl WeaponLoadout {
    pub fn new() -> Self {
        let weapon_ids = all_weapon_ids();
        assert!(!weapon_ids.is_empty(), "weapon catalog is empty");
        Self {
            slots: weapon_ids.into_iter().map(WeaponState::new).collect(),
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
                BuyStage::ClassSelect | BuyStage::WeaponSelect { .. } => BuyStage::Closed,
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
                let classes = weapon_classes();
                if let Some(class) = classes.get(choice - 1).copied() {
                    self.buy_stage = BuyStage::WeaponSelect { class, page: 0 };
                }
            }
            BuyStage::WeaponSelect { class, page } => {
                let class_weapons = weapon_ids_for_class(class);
                let page_start = page * WEAPON_PAGE_SIZE;
                match choice {
                    1..=3 => {
                        let weapon_index = page_start + (choice - 1);
                        if let Some(weapon) = class_weapons.get(weapon_index) {
                            if let Some(slot_idx) =
                                self.slots.iter().position(|slot| slot.id() == *weapon)
                            {
                                self.active_slot = slot_idx;
                            }
                            self.buy_stage = BuyStage::Closed;
                        }
                    }
                    4 => {
                        if page > 0 {
                            self.buy_stage = BuyStage::WeaponSelect {
                                class,
                                page: page - 1,
                            };
                        }
                    }
                    5 => {
                        if page_start + WEAPON_PAGE_SIZE < class_weapons.len() {
                            self.buy_stage = BuyStage::WeaponSelect {
                                class,
                                page: page + 1,
                            };
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn buy_prompt(&self) -> Option<String> {
        match self.buy_stage {
            BuyStage::Closed => None,
            BuyStage::ClassSelect => Some(class_select_prompt()),
            BuyStage::WeaponSelect { class, page } => Some(weapon_select_prompt(class, page)),
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

fn class_select_prompt() -> String {
    let classes = weapon_classes();
    let mut out = String::from("BUY: choose class");
    for (idx, class) in classes.iter().take(5).enumerate() {
        out.push_str(&format!("  {}={}", idx + 1, class.label()));
    }
    out.push_str("  (B cancel)");
    out
}

fn weapon_select_prompt(class: WeaponClass, page: usize) -> String {
    let class_weapons = weapon_ids_for_class(class);
    if class_weapons.is_empty() {
        return format!("{}: no weapons found  (B cancel)", class.label());
    }

    let page_count = class_weapons.len().div_ceil(WEAPON_PAGE_SIZE);
    let clamped_page = page.min(page_count.saturating_sub(1));
    let page_start = clamped_page * WEAPON_PAGE_SIZE;
    let page_num = clamped_page + 1;

    let slot_name = |slot_index: usize| -> String {
        class_weapons
            .get(page_start + slot_index)
            .map(|weapon| weapon.stats().name.to_string())
            .unwrap_or_else(|| "---".to_string())
    };

    let can_prev = clamped_page > 0;
    let can_next = page_start + WEAPON_PAGE_SIZE < class_weapons.len();

    format!(
        "{} p{}/{}: 1={}  2={}  3={}  4={}  5={}  (B cancel)",
        class.label(),
        page_num,
        page_count,
        slot_name(0),
        slot_name(1),
        slot_name(2),
        if can_prev { "Prev" } else { "---" },
        if can_next { "Next" } else { "---" },
    )
}
