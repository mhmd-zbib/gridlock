use crate::core::entity::weapon::{WeaponClass, WeaponStats};

pub fn stats() -> WeaponStats {
    WeaponStats {
        class: WeaponClass::Rifle,
        name: "AK-47",
        visibility_range: 320.0,
        visibility_half_angle_deg: 40.0,
        aim_cone_render_range: 220.0,
        aim_base_half_angle_deg: 1.4,
        movement_spread_max_deg: 12.5,
        bullet_speed: 780.0,
        bullet_damage: 1,
        recoil_per_shot_deg: 3.6,
        recoil_max_deg: 22.0,
        recoil_decay_deg_per_sec: 8.0,
        fire_rate_rps: 10.0,
        mag_size: 30,
        reload_time_secs: 2.4,
    }
}
