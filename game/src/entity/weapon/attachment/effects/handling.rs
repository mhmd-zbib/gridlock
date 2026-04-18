use serde::{Deserialize, Serialize};

use super::one;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct HandlingEffects {
    #[serde(default = "one")]
    pub ads_time_mult: f32,
    #[serde(default = "one")]
    pub weapon_swap_time_mult: f32,
    #[serde(default = "one")]
    pub reload_time_mult: f32,
    #[serde(default = "one")]
    pub sprint_to_fire_time_mult: f32,
    #[serde(default = "one")]
    pub mag_size_mult: f32,
}

impl Default for HandlingEffects {
    fn default() -> Self {
        Self {
            ads_time_mult: 1.0,
            weapon_swap_time_mult: 1.0,
            reload_time_mult: 1.0,
            sprint_to_fire_time_mult: 1.0,
            mag_size_mult: 1.0,
        }
    }
}

impl HandlingEffects {
    pub(super) fn combine(self, other: Self) -> Self {
        Self {
            ads_time_mult: self.ads_time_mult * other.ads_time_mult,
            weapon_swap_time_mult: self.weapon_swap_time_mult * other.weapon_swap_time_mult,
            reload_time_mult: self.reload_time_mult * other.reload_time_mult,
            sprint_to_fire_time_mult: self.sprint_to_fire_time_mult
                * other.sprint_to_fire_time_mult,
            mag_size_mult: self.mag_size_mult * other.mag_size_mult,
        }
    }
}
