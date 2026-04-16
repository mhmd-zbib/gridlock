use serde::{Deserialize, Serialize};

use crate::entity::weapon::WeaponStats;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AttachmentEffects {
    #[serde(default)]
    pub recoil: RecoilEffects,
    #[serde(default)]
    pub handling: HandlingEffects,
    #[serde(default)]
    pub accuracy: AccuracyEffects,
    #[serde(default)]
    pub power: PowerEffects,
    #[serde(default)]
    pub stealth: StealthEffects,
    #[serde(default)]
    pub mobility: MobilityEffects,
    #[serde(default)]
    pub utility: UtilityEffects,
}

impl Default for AttachmentEffects {
    fn default() -> Self {
        Self {
            recoil: RecoilEffects::default(),
            handling: HandlingEffects::default(),
            accuracy: AccuracyEffects::default(),
            power: PowerEffects::default(),
            stealth: StealthEffects::default(),
            mobility: MobilityEffects::default(),
            utility: UtilityEffects::default(),
        }
    }
}

impl AttachmentEffects {
    pub fn combine(self, other: Self) -> Self {
        Self {
            recoil: self.recoil.combine(other.recoil),
            handling: self.handling.combine(other.handling),
            accuracy: self.accuracy.combine(other.accuracy),
            power: self.power.combine(other.power),
            stealth: self.stealth.combine(other.stealth),
            mobility: self.mobility.combine(other.mobility),
            utility: self.utility.combine(other.utility),
        }
    }

    pub fn apply_to_stats(self, mut stats: WeaponStats) -> WeaponStats {
        // Recoil control
        stats.recoil_per_shot_deg *= self.recoil.vertical_recoil_mult;
        stats.recoil_max_deg *= self.recoil.vertical_recoil_mult;
        stats.recoil_decay_deg_per_sec *= self.recoil.recoil_recovery_mult;

        // Handling
        stats.reload_time_secs *= self.handling.reload_time_mult;
        stats.ads_time_secs *= self.handling.ads_time_mult;
        stats.weapon_swap_time_secs *= self.handling.weapon_swap_time_mult;
        stats.sprint_to_fire_time_secs *= self.handling.sprint_to_fire_time_mult;
        stats.mag_size = ((stats.mag_size as f32) * self.handling.mag_size_mult)
            .round()
            .max(1.0) as u32;

        // Accuracy / spread
        stats.aim_base_half_angle_deg *= self.accuracy.first_shot_spread_mult;
        stats.movement_spread_max_deg *= self.accuracy.movement_spread_mult;
        stats.hip_fire_spread_deg *= self.accuracy.hip_fire_spread_mult;

        // Range / damage
        stats.bullet_speed *= self.power.bullet_velocity_mult;
        stats.damage_falloff_range *= self.power.damage_falloff_range_mult;
        stats.penetration_power *= self.power.penetration_power_mult;
        stats.bullet_damage = ((stats.bullet_damage as f32) * self.power.bullet_damage_mult)
            .round()
            .max(1.0) as u32;

        // Stealth / visibility
        stats.shot_sound_radius *= self.stealth.shot_sound_radius_mult;
        stats.minimap_signature *= self.stealth.minimap_signature_mult;
        stats.muzzle_flash_intensity *= self.stealth.muzzle_flash_mult;

        // Mobility
        stats.ads_move_speed_mult *= self.mobility.ads_move_speed_mult;
        stats.strafe_spread_penalty_mult *= self.mobility.strafe_spread_penalty_mult;
        stats.jump_stability_mult *= self.mobility.jump_stability_mult;

        // Utility
        stats.ping_visibility_mult *= self.utility.ping_visibility_mult;
        stats.detection_strength_mult *= self.utility.detection_strength_mult;
        stats.enemy_mark_duration_mult *= self.utility.enemy_mark_duration_mult;

        stats
    }
}

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

pub(super) fn one() -> f32 {
    1.0
}

pub(super) fn effects_are_valid(effects: AttachmentEffects) -> bool {
    let values = [
        effects.recoil.vertical_recoil_mult,
        effects.recoil.horizontal_recoil_mult,
        effects.recoil.recoil_recovery_mult,
        effects.handling.ads_time_mult,
        effects.handling.weapon_swap_time_mult,
        effects.handling.reload_time_mult,
        effects.handling.sprint_to_fire_time_mult,
        effects.handling.mag_size_mult,
        effects.accuracy.movement_spread_mult,
        effects.accuracy.hip_fire_spread_mult,
        effects.accuracy.first_shot_spread_mult,
        effects.power.damage_falloff_range_mult,
        effects.power.bullet_velocity_mult,
        effects.power.penetration_power_mult,
        effects.power.bullet_damage_mult,
        effects.stealth.shot_sound_radius_mult,
        effects.stealth.minimap_signature_mult,
        effects.stealth.muzzle_flash_mult,
        effects.mobility.ads_move_speed_mult,
        effects.mobility.strafe_spread_penalty_mult,
        effects.mobility.jump_stability_mult,
        effects.utility.ping_visibility_mult,
        effects.utility.detection_strength_mult,
        effects.utility.enemy_mark_duration_mult,
    ];
    values.into_iter().all(|v| v >= 0.0 && v.is_finite())
}
