use crate::core::entity::weapon::{WeaponClass, WeaponId, WeaponStats};
use crate::core::world::units::px_to_tiles;
use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Deserialize)]
struct WeaponStatsJson<'a> {
    class: WeaponClass,
    name: &'a str,
    visibility_range_px: f32,
    visibility_half_angle_deg: f32,
    aim_cone_render_range_px: f32,
    aim_base_half_angle_deg: f32,
    movement_spread_max_deg: f32,
    bullet_speed_px_per_sec: f32,
    bullet_damage: u32,
    recoil_per_shot_deg: f32,
    recoil_max_deg: f32,
    recoil_decay_deg_per_sec: f32,
    fire_rate_rps: f32,
    mag_size: u32,
    reload_time_secs: f32,
}

fn load_stats(json: &'static str) -> WeaponStats {
    let parsed: WeaponStatsJson<'static> =
        serde_json::from_str(json).unwrap_or_else(|e| panic!("invalid weapon json: {e}"));
    WeaponStats {
        class: parsed.class,
        name: parsed.name,
        visibility_range: px_to_tiles(parsed.visibility_range_px),
        visibility_half_angle_deg: parsed.visibility_half_angle_deg,
        aim_cone_render_range: px_to_tiles(parsed.aim_cone_render_range_px),
        aim_base_half_angle_deg: parsed.aim_base_half_angle_deg,
        movement_spread_max_deg: parsed.movement_spread_max_deg,
        bullet_speed: px_to_tiles(parsed.bullet_speed_px_per_sec),
        bullet_damage: parsed.bullet_damage,
        recoil_per_shot_deg: parsed.recoil_per_shot_deg,
        recoil_max_deg: parsed.recoil_max_deg,
        recoil_decay_deg_per_sec: parsed.recoil_decay_deg_per_sec,
        fire_rate_rps: parsed.fire_rate_rps,
        mag_size: parsed.mag_size,
        reload_time_secs: parsed.reload_time_secs,
    }
}

pub fn stats(id: WeaponId) -> WeaponStats {
    match id {
        WeaponId::Ak47 => ak47_stats(),
        WeaponId::Mp5 => mp5_stats(),
        WeaponId::Sniper => sniper_stats(),
        WeaponId::M4a1 => m4a1_stats(),
        WeaponId::Uzi => uzi_stats(),
        WeaponId::Dmr => dmr_stats(),
    }
}

fn ak47_stats() -> WeaponStats {
    static STATS: OnceLock<WeaponStats> = OnceLock::new();
    *STATS.get_or_init(|| {
        load_stats(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/weapons/ak47.json"
        )))
    })
}

fn m4a1_stats() -> WeaponStats {
    static STATS: OnceLock<WeaponStats> = OnceLock::new();
    *STATS.get_or_init(|| {
        load_stats(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/weapons/m4a1.json"
        )))
    })
}

fn mp5_stats() -> WeaponStats {
    static STATS: OnceLock<WeaponStats> = OnceLock::new();
    *STATS.get_or_init(|| {
        load_stats(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/weapons/mp5.json"
        )))
    })
}

fn uzi_stats() -> WeaponStats {
    static STATS: OnceLock<WeaponStats> = OnceLock::new();
    *STATS.get_or_init(|| {
        load_stats(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/weapons/uzi.json"
        )))
    })
}

fn dmr_stats() -> WeaponStats {
    static STATS: OnceLock<WeaponStats> = OnceLock::new();
    *STATS.get_or_init(|| {
        load_stats(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/weapons/dmr.json"
        )))
    })
}

fn sniper_stats() -> WeaponStats {
    static STATS: OnceLock<WeaponStats> = OnceLock::new();
    *STATS.get_or_init(|| {
        load_stats(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/weapons/sniper.json"
        )))
    })
}
