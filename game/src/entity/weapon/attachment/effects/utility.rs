use serde::{Deserialize, Serialize};

use super::one;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct UtilityEffects {
    #[serde(default = "one")]
    pub ping_visibility_mult: f32,
    #[serde(default = "one")]
    pub detection_strength_mult: f32,
    #[serde(default = "one")]
    pub enemy_mark_duration_mult: f32,
}

impl Default for UtilityEffects {
    fn default() -> Self {
        Self {
            ping_visibility_mult: 1.0,
            detection_strength_mult: 1.0,
            enemy_mark_duration_mult: 1.0,
        }
    }
}

impl UtilityEffects {
    pub(super) fn combine(self, other: Self) -> Self {
        Self {
            ping_visibility_mult: self.ping_visibility_mult * other.ping_visibility_mult,
            detection_strength_mult: self.detection_strength_mult * other.detection_strength_mult,
            enemy_mark_duration_mult: self.enemy_mark_duration_mult
                * other.enemy_mark_duration_mult,
        }
    }
}
