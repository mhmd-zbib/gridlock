use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::entity::weapon::attachment::{AttachmentCategory, AttachmentId, attachment_id_by_id};
use crate::entity::weapon::{WeaponClass, WeaponStats};
use crate::world::units::px_to_tiles;

use super::WeaponEntry;
use super::defaults::{
    base_ads_time_secs, base_hip_fire_spread_deg, base_penetration_power, class_sort_key,
};

#[derive(Deserialize)]
pub(super) struct WeaponStatsJson {
    pub id: String,
    pub class: WeaponClass,
    pub name: String,
    pub attachments: WeaponAttachmentMapJson,
    pub visibility_range_px: f32,
    pub visibility_half_angle_deg: f32,
    pub aim_cone_render_range_px: f32,
    pub aim_base_half_angle_deg: f32,
    pub movement_spread_max_deg: f32,
    pub bullet_speed_px_per_sec: f32,
    pub bullet_damage: u32,
    pub recoil_per_shot_deg: f32,
    pub recoil_max_deg: f32,
    pub recoil_decay_deg_per_sec: f32,
    pub fire_rate_rps: f32,
    pub mag_size: u32,
    pub reload_time_secs: f32,
}

#[derive(Deserialize, Default)]
pub(super) struct WeaponAttachmentMapJson {
    #[serde(default)]
    pub optic: Vec<String>,
    #[serde(default)]
    pub barrel: Vec<String>,
    #[serde(default)]
    pub grip: Vec<String>,
    #[serde(default)]
    pub magazine: Vec<String>,
    #[serde(default)]
    pub stock: Vec<String>,
    #[serde(default)]
    pub underbarrel: Vec<String>,
}

pub(super) fn load_catalog() -> Vec<WeaponEntry> {
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

    let mut seen_ids = HashSet::new();
    for entry in &entries {
        let normalized = entry.id.to_ascii_lowercase();
        assert!(
            seen_ids.insert(normalized),
            "duplicate weapon id '{}'",
            entry.id
        );
    }
    entries
}

pub(super) fn resolve_weapons_dir() -> PathBuf {
    crate::resolve_config_dir("weapons")
        .expect("weapons directory not found — expected 'assets/configs/weapons' next to the executable or in the working directory")
}

pub(super) fn load_weapon_file(path: &Path) -> WeaponEntry {
    let json = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read weapon file '{}': {e}", path.display()));
    let parsed: WeaponStatsJson = serde_json::from_str(&json)
        .unwrap_or_else(|e| panic!("invalid weapon json '{}': {e}", path.display()));

    let WeaponStatsJson {
        id,
        class,
        name,
        attachments,
        visibility_range_px,
        visibility_half_angle_deg,
        aim_cone_render_range_px,
        aim_base_half_angle_deg,
        movement_spread_max_deg,
        bullet_speed_px_per_sec,
        bullet_damage,
        recoil_per_shot_deg,
        recoil_max_deg,
        recoil_decay_deg_per_sec,
        fire_rate_rps,
        mag_size,
        reload_time_secs,
    } = parsed;

    if !is_snake_case_id(&id) {
        panic!(
            "weapon file '{}' has non-snake-case id '{}'",
            path.display(),
            id
        );
    }

    let attachment_optic =
        parse_category_attachment_ids(path, AttachmentCategory::Optic, attachments.optic);
    let attachment_barrel =
        parse_category_attachment_ids(path, AttachmentCategory::Barrel, attachments.barrel);
    let attachment_grip =
        parse_category_attachment_ids(path, AttachmentCategory::Grip, attachments.grip);
    let attachment_magazine =
        parse_category_attachment_ids(path, AttachmentCategory::Magazine, attachments.magazine);
    let attachment_stock =
        parse_category_attachment_ids(path, AttachmentCategory::Stock, attachments.stock);
    let attachment_underbarrel = parse_category_attachment_ids(
        path,
        AttachmentCategory::Underbarrel,
        attachments.underbarrel,
    );

    let attachment_lists = [
        attachment_optic,
        attachment_barrel,
        attachment_grip,
        attachment_magazine,
        attachment_stock,
        attachment_underbarrel,
    ];

    let mut attachment_categories = Vec::new();
    for category in AttachmentCategory::all() {
        if !attachment_lists[category.index()].is_empty() {
            attachment_categories.push(*category);
        }
    }
    if attachment_categories.is_empty() {
        panic!(
            "weapon file '{}' has no supported attachments in 'attachments' map",
            path.display()
        );
    }

    WeaponEntry {
        id: leak_string(id),
        stats: WeaponStats {
            class,
            name: leak_string(name),
            visibility_range: px_to_tiles(visibility_range_px),
            visibility_half_angle_deg,
            aim_cone_render_range: px_to_tiles(aim_cone_render_range_px),
            aim_base_half_angle_deg,
            movement_spread_max_deg,
            bullet_speed: px_to_tiles(bullet_speed_px_per_sec),
            bullet_damage,
            recoil_per_shot_deg,
            recoil_max_deg,
            recoil_decay_deg_per_sec,
            fire_rate_rps,
            mag_size,
            reload_time_secs,
            ads_time_secs: base_ads_time_secs(class),
            weapon_swap_time_secs: 0.7,
            sprint_to_fire_time_secs: 0.2,
            hip_fire_spread_deg: base_hip_fire_spread_deg(class),
            damage_falloff_range: px_to_tiles(visibility_range_px),
            penetration_power: base_penetration_power(class),
            shot_sound_radius: px_to_tiles(visibility_range_px * 1.1),
            minimap_signature: 1.0,
            muzzle_flash_intensity: 1.0,
            ads_move_speed_mult: 1.0,
            strafe_spread_penalty_mult: 1.0,
            jump_stability_mult: 1.0,
            ping_visibility_mult: 1.0,
            detection_strength_mult: 1.0,
            enemy_mark_duration_mult: 1.0,
        },
        attachments: attachment_lists,
        attachment_categories,
    }
}

pub(super) fn parse_category_attachment_ids(
    path: &Path,
    category: AttachmentCategory,
    ids: Vec<String>,
) -> Vec<AttachmentId> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    for id in ids {
        if !is_snake_case_id(&id) {
            panic!(
                "weapon file '{}' has non-snake-case attachment id '{}' in category {:?}",
                path.display(),
                id,
                category
            );
        }

        let normalized = id.to_ascii_lowercase();
        if !seen.insert(normalized) {
            panic!(
                "weapon file '{}' has duplicate attachment id '{}' in category {:?}",
                path.display(),
                id,
                category
            );
        }

        let Some(attachment) = attachment_id_by_id(&id) else {
            panic!(
                "weapon file '{}' references unknown attachment id '{}' in category {:?}",
                path.display(),
                id,
                category
            );
        };
        if attachment.category() != category {
            panic!(
                "weapon file '{}' has attachment id '{}' in category {:?}, but attachment belongs to {:?}",
                path.display(),
                id,
                category,
                attachment.category()
            );
        }
        out.push(attachment);
    }

    out
}

pub(super) fn leak_string(value: String) -> &'static str {
    Box::leak(value.into_boxed_str())
}

pub(super) fn is_snake_case_id(id: &str) -> bool {
    if id.is_empty() {
        return false;
    }
    let mut chars = id.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() {
        return false;
    }
    chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
}
