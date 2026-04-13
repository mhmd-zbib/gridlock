use crate::core::entity::weapon::{WeaponClass, WeaponStats};
use crate::core::world::units::px_to_tiles;

pub fn stats() -> WeaponStats {
    WeaponStats {
        class: WeaponClass::Sniper,
        name: "DMR",
        visibility_range: px_to_tiles(520.0),
        visibility_half_angle_deg: 28.0,
        aim_cone_render_range: px_to_tiles(360.0),
        aim_base_half_angle_deg: 0.75,
        movement_spread_max_deg: 45.0,
        bullet_speed: px_to_tiles(980.0),
        bullet_damage: 2,
        recoil_per_shot_deg: 5.0,
        recoil_max_deg: 24.0,
        recoil_decay_deg_per_sec: 6.5,
        fire_rate_rps: 2.6,
        mag_size: 12,
        reload_time_secs: 2.6,
    }
}
