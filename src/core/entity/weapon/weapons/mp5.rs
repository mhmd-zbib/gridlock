use crate::core::entity::weapon::{WeaponClass, WeaponStats};
use crate::core::world::units::px_to_tiles;

pub fn stats() -> WeaponStats {
    WeaponStats {
        class: WeaponClass::Smg,
        name: "MP5",
        visibility_range: px_to_tiles(270.0),
        visibility_half_angle_deg: 40.0,
        aim_cone_render_range: px_to_tiles(150.0),
        aim_base_half_angle_deg: 2.0,
        movement_spread_max_deg: 9.5,
        bullet_speed: px_to_tiles(620.0),
        bullet_damage: 1,
        recoil_per_shot_deg: 2.1,
        recoil_max_deg: 15.0,
        recoil_decay_deg_per_sec: 11.0,
        fire_rate_rps: 13.0,
        mag_size: 30,
        reload_time_secs: 1.9,
    }
}
