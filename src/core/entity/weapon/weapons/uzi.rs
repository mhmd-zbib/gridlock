use crate::core::entity::weapon::{WeaponClass, WeaponStats};
use crate::core::world::units::px_to_tiles;

pub fn stats() -> WeaponStats {
    WeaponStats {
        class: WeaponClass::Smg,
        name: "Uzi",
        visibility_range: px_to_tiles(235.0),
        visibility_half_angle_deg: 42.0,
        aim_cone_render_range: px_to_tiles(130.0),
        aim_base_half_angle_deg: 2.6,
        movement_spread_max_deg: 10.0,
        bullet_speed: px_to_tiles(560.0),
        bullet_damage: 1,
        recoil_per_shot_deg: 1.7,
        recoil_max_deg: 14.0,
        recoil_decay_deg_per_sec: 12.5,
        fire_rate_rps: 16.0,
        mag_size: 32,
        reload_time_secs: 1.7,
    }
}
