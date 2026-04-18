use crate::entity::weapon::WeaponClass;

pub(super) fn class_sort_key(class: WeaponClass) -> u8 {
    match class {
        WeaponClass::Rifle => 0,
        WeaponClass::Smg => 1,
        WeaponClass::Sniper => 2,
    }
}

pub(super) fn base_ads_time_secs(class: WeaponClass) -> f32 {
    match class {
        WeaponClass::Rifle => 0.24,
        WeaponClass::Smg => 0.2,
        WeaponClass::Sniper => 0.32,
    }
}

pub(super) fn base_hip_fire_spread_deg(class: WeaponClass) -> f32 {
    match class {
        WeaponClass::Rifle => 4.8,
        WeaponClass::Smg => 3.8,
        WeaponClass::Sniper => 6.5,
    }
}

pub(super) fn base_penetration_power(class: WeaponClass) -> f32 {
    match class {
        WeaponClass::Rifle => 1.0,
        WeaponClass::Smg => 0.8,
        WeaponClass::Sniper => 1.35,
    }
}
