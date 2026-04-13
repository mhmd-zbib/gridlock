use crate::core::entity::weapon::{WeaponClass, WeaponStats};
use crate::core::world::units::px_to_tiles;

pub fn stats() -> WeaponStats {
    WeaponStats {
        class: WeaponClass::Rifle,
        name: "M4A1",
        visibility_range: px_to_tiles(360.0),
        visibility_half_angle_deg: 38.0,
        aim_cone_render_range: px_to_tiles(260.0),
        aim_base_half_angle_deg: 1.2,
        movement_spread_max_deg: 10.5,
        bullet_speed: px_to_tiles(840.0),
        bullet_damage: 1,
        recoil_per_shot_deg: 3.0,
        recoil_max_deg: 19.0,
        recoil_decay_deg_per_sec: 8.8,
        fire_rate_rps: 9.2,
        mag_size: 30,
        reload_time_secs: 2.2,
    }
}
