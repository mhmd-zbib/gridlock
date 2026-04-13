mod class;
mod id;
mod state;
mod stats;
mod weapons;

pub use class::WeaponClass;
pub use id::WeaponId;
pub use state::WeaponState;
pub use stats::WeaponStats;

pub fn all_weapon_ids() -> Vec<WeaponId> {
    weapons::all_ids()
}

pub fn weapon_ids_for_class(class: WeaponClass) -> Vec<WeaponId> {
    weapons::ids_for_class(class)
}

pub fn weapon_classes() -> Vec<WeaponClass> {
    weapons::available_classes()
}
