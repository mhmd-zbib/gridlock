mod accuracy;
mod handling;
mod mobility;
mod power;
mod recoil;
mod stealth;
mod utility;

pub use accuracy::AccuracyEffects;
pub use handling::HandlingEffects;
pub use mobility::MobilityEffects;
pub use power::PowerEffects;
pub use recoil::RecoilEffects;
pub use stealth::StealthEffects;
pub use utility::UtilityEffects;

use crate::entity::weapon::WeaponStats;

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
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
