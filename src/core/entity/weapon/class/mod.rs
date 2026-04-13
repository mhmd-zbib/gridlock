mod rifle;
mod smg;
mod sniper;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeaponClass {
    Rifle,
    Smg,
    Sniper,
}

impl WeaponClass {
    pub fn label(self) -> &'static str {
        match self {
            WeaponClass::Rifle => rifle::LABEL,
            WeaponClass::Smg => smg::LABEL,
            WeaponClass::Sniper => sniper::LABEL,
        }
    }
}
