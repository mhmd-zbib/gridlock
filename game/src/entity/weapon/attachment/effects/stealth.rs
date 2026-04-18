use serde::{Deserialize, Serialize};

use super::one;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct StealthEffects {
    #[serde(default = "one")]
    pub shot_sound_radius_mult: f32,
    #[serde(default = "one")]
    pub minimap_signature_mult: f32,
    #[serde(default = "one")]
    pub muzzle_flash_mult: f32,
}

impl Default for StealthEffects {
    fn default() -> Self {
        Self {
            shot_sound_radius_mult: 1.0,
            minimap_signature_mult: 1.0,
            muzzle_flash_mult: 1.0,
        }
    }
}

impl StealthEffects {
    pub(super) fn combine(self, other: Self) -> Self {
        Self {
            shot_sound_radius_mult: self.shot_sound_radius_mult * other.shot_sound_radius_mult,
            minimap_signature_mult: self.minimap_signature_mult * other.minimap_signature_mult,
            muzzle_flash_mult: self.muzzle_flash_mult * other.muzzle_flash_mult,
        }
    }
}
