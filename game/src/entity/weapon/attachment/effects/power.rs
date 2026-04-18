use serde::{Deserialize, Serialize};

use super::one;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PowerEffects {
    #[serde(default = "one")]
    pub damage_falloff_range_mult: f32,
    #[serde(default = "one")]
    pub bullet_velocity_mult: f32,
    #[serde(default = "one")]
    pub penetration_power_mult: f32,
    #[serde(default = "one")]
    pub bullet_damage_mult: f32,
}

impl Default for PowerEffects {
    fn default() -> Self {
        Self {
            damage_falloff_range_mult: 1.0,
            bullet_velocity_mult: 1.0,
            penetration_power_mult: 1.0,
            bullet_damage_mult: 1.0,
        }
    }
}

impl PowerEffects {
    pub(super) fn combine(self, other: Self) -> Self {
        Self {
            damage_falloff_range_mult: self.damage_falloff_range_mult
                * other.damage_falloff_range_mult,
            bullet_velocity_mult: self.bullet_velocity_mult * other.bullet_velocity_mult,
            penetration_power_mult: self.penetration_power_mult * other.penetration_power_mult,
            bullet_damage_mult: self.bullet_damage_mult * other.bullet_damage_mult,
        }
    }
}
