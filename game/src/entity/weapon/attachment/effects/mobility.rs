use serde::{Deserialize, Serialize};

use super::one;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MobilityEffects {
    #[serde(default = "one")]
    pub ads_move_speed_mult: f32,
    #[serde(default = "one")]
    pub strafe_spread_penalty_mult: f32,
    #[serde(default = "one")]
    pub jump_stability_mult: f32,
}

impl Default for MobilityEffects {
    fn default() -> Self {
        Self {
            ads_move_speed_mult: 1.0,
            strafe_spread_penalty_mult: 1.0,
            jump_stability_mult: 1.0,
        }
    }
}

impl MobilityEffects {
    pub(super) fn combine(self, other: Self) -> Self {
        Self {
            ads_move_speed_mult: self.ads_move_speed_mult * other.ads_move_speed_mult,
            strafe_spread_penalty_mult: self.strafe_spread_penalty_mult
                * other.strafe_spread_penalty_mult,
            jump_stability_mult: self.jump_stability_mult * other.jump_stability_mult,
        }
    }
}
