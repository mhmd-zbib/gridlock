use serde::{Deserialize, Serialize};

use super::one;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RecoilEffects {
    #[serde(default = "one")]
    pub vertical_recoil_mult: f32,
    #[serde(default = "one")]
    pub horizontal_recoil_mult: f32,
    #[serde(default = "one")]
    pub recoil_recovery_mult: f32,
}

impl Default for RecoilEffects {
    fn default() -> Self {
        Self {
            vertical_recoil_mult: 1.0,
            horizontal_recoil_mult: 1.0,
            recoil_recovery_mult: 1.0,
        }
    }
}

impl RecoilEffects {
    pub(super) fn combine(self, other: Self) -> Self {
        Self {
            vertical_recoil_mult: self.vertical_recoil_mult * other.vertical_recoil_mult,
            horizontal_recoil_mult: self.horizontal_recoil_mult * other.horizontal_recoil_mult,
            recoil_recovery_mult: self.recoil_recovery_mult * other.recoil_recovery_mult,
        }
    }
}
