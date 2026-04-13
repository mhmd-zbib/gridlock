use crate::core::entity::weapon::{WeaponClass, WeaponId, WeaponStats};
use crate::core::world::units::px_to_tiles;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

#[derive(Deserialize)]
struct WeaponStatsJson {
    class: WeaponClass,
    name: String,
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

#[derive(Clone, Copy)]
struct WeaponEntry {
    id: &'static str,
    stats: WeaponStats,
}

static CATALOG: OnceLock<Vec<WeaponEntry>> = OnceLock::new();

pub fn stats(id: WeaponId) -> WeaponStats {
    catalog()
        .get(id.0)
        .unwrap_or_else(|| panic!("invalid weapon id index: {}", id.0))
        .stats
}

pub fn all_ids() -> Vec<WeaponId> {
    (0..catalog().len()).map(WeaponId).collect()
}

pub fn ids_for_class(class: WeaponClass) -> Vec<WeaponId> {
    catalog()
        .iter()
        .enumerate()
        .filter_map(|(idx, entry)| (entry.stats.class == class).then_some(WeaponId(idx)))
        .collect()
}

pub fn available_classes() -> Vec<WeaponClass> {
    let mut out = Vec::new();
    for entry in catalog() {
        if !out.contains(&entry.stats.class) {
            out.push(entry.stats.class);
        }
    }
    out
}

pub fn id_by_name(name: &str) -> Option<WeaponId> {
    catalog()
        .iter()
        .position(|entry| entry.stats.name == name || entry.id.eq_ignore_ascii_case(name))
        .map(WeaponId)
}

fn catalog() -> &'static [WeaponEntry] {
    CATALOG.get_or_init(load_catalog).as_slice()
}

fn load_catalog() -> Vec<WeaponEntry> {
    let weapons_dir = resolve_weapons_dir();
    let read_dir = fs::read_dir(&weapons_dir).unwrap_or_else(|e| {
        panic!(
            "failed to read weapons directory '{}': {e}",
            weapons_dir.display()
        )
    });

    let mut entries: Vec<WeaponEntry> = read_dir
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .map(|path| load_weapon_file(&path))
        .collect();

    if entries.is_empty() {
        panic!(
            "no weapon JSON files found in weapons directory '{}'",
            weapons_dir.display()
        );
    }

    entries.sort_by(|a, b| {
        class_sort_key(a.stats.class)
            .cmp(&class_sort_key(b.stats.class))
            .then_with(|| a.stats.name.cmp(b.stats.name))
    });
    entries
}

fn resolve_weapons_dir() -> PathBuf {
    let cwd_weapons = std::env::current_dir()
        .ok()
        .map(|cwd| cwd.join("weapons"))
        .filter(|path| path.is_dir());
    if let Some(path) = cwd_weapons {
        return path;
    }

    let manifest_weapons = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("weapons");
    if manifest_weapons.is_dir() {
        return manifest_weapons;
    }

    panic!(
        "weapons directory not found (checked './weapons' and '{}/weapons')",
        env!("CARGO_MANIFEST_DIR")
    );
}

fn load_weapon_file(path: &Path) -> WeaponEntry {
    let json = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read weapon file '{}': {e}", path.display()));
    let parsed: WeaponStatsJson = serde_json::from_str(&json)
        .unwrap_or_else(|e| panic!("invalid weapon json '{}': {e}", path.display()));

    let file_stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_else(|| panic!("weapon file '{}' has invalid name", path.display()))
        .to_string();

    WeaponEntry {
        id: leak_string(file_stem),
        stats: WeaponStats {
            class: parsed.class,
            name: leak_string(parsed.name),
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
        },
    }
}

fn leak_string(value: String) -> &'static str {
    Box::leak(value.into_boxed_str())
}

fn class_sort_key(class: WeaponClass) -> u8 {
    match class {
        WeaponClass::Rifle => 0,
        WeaponClass::Smg => 1,
        WeaponClass::Sniper => 2,
    }
}
