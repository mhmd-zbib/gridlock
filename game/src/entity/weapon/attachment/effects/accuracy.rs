use serde::{Deserialize, Serialize};

use super::one;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AccuracyEffects {
    #[serde(default = "one")]
    pub movement_spread_mult: f32,
    #[serde(default = "one")]
    pub hip_fire_spread_mult: f32,
    #[serde(default = "one")]
    pub first_shot_spread_mult: f32,
}

impl Default for AccuracyEffects {
    fn default() -> Self {
        Self {
            movement_spread_mult: 1.0,
            hip_fire_spread_mult: 1.0,
            first_shot_spread_mult: 1.0,
        }
    }
}

impl AccuracyEffects {
    pub(super) fn combine(self, other: Self) -> Self {
        Self {
            movement_spread_mult: self.movement_spread_mult * other.movement_spread_mult,
            hip_fire_spread_mult: self.hip_fire_spread_mult * other.hip_fire_spread_mult,
            first_shot_spread_mult: self.first_shot_spread_mult * other.first_shot_spread_mult,
        }
    }
}
