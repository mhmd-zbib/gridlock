use crate::entity::weapon::attachment::{
    ATTACHMENT_CATEGORY_COUNT, AttachmentCategory, AttachmentId, attachment_id_by_id,
};
use crate::entity::weapon::{WeaponClass, WeaponId, WeaponStats};
use crate::world::units::px_to_tiles;
use serde::Deserialize;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

#[derive(Deserialize)]
struct WeaponStatsJson {
    id: String,
    class: WeaponClass,
    name: String,
    attachments: WeaponAttachmentMapJson,
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

#[derive(Deserialize, Default)]
struct WeaponAttachmentMapJson {
    #[serde(default)]
    optic: Vec<String>,
    #[serde(default)]
    barrel: Vec<String>,
    #[serde(default)]
    grip: Vec<String>,
    #[serde(default)]
    magazine: Vec<String>,
    #[serde(default)]
    stock: Vec<String>,
    #[serde(default)]
    underbarrel: Vec<String>,
}

#[derive(Clone)]
struct WeaponEntry {
    id: &'static str,
    stats: WeaponStats,
    attachments: [Vec<AttachmentId>; ATTACHMENT_CATEGORY_COUNT],
    attachment_categories: Vec<AttachmentCategory>,
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

pub fn attachment_categories(id: WeaponId) -> &'static [AttachmentCategory] {
    catalog()
        .get(id.0)
        .unwrap_or_else(|| panic!("invalid weapon id index: {}", id.0))
        .attachment_categories
        .as_slice()
}

pub fn attachment_ids_for_category(
    id: WeaponId,
    category: AttachmentCategory,
) -> &'static [AttachmentId] {
    catalog()
        .get(id.0)
        .unwrap_or_else(|| panic!("invalid weapon id index: {}", id.0))
        .attachments[category.index()]
    .as_slice()
}

pub fn supports_attachment_category(id: WeaponId, category: AttachmentCategory) -> bool {
    !attachment_ids_for_category(id, category).is_empty()
}

pub fn supports_attachment(id: WeaponId, attachment: &AttachmentId) -> bool {
    attachment_ids_for_category(id, attachment.category())
        .iter()
        .any(|allowed| allowed == attachment)
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

fn resolve_weapons_dir() -> PathBuf {
    let cwd_weapons = std::env::current_dir()
        .ok()
        .map(|cwd| cwd.join("assets").join("weapons"))
        .filter(|path| path.is_dir());
    if let Some(path) = cwd_weapons {
        return path;
    }

    let manifest_weapons = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("weapons");
    if manifest_weapons.is_dir() {
        return manifest_weapons;
    }

    panic!(
        "weapons directory not found (checked './assets/weapons' and '{}/assets/weapons')",
        env!("CARGO_MANIFEST_DIR")
    );
}

fn load_weapon_file(path: &Path) -> WeaponEntry {
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

fn parse_category_attachment_ids(
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

fn base_ads_time_secs(class: WeaponClass) -> f32 {
    match class {
        WeaponClass::Rifle => 0.24,
        WeaponClass::Smg => 0.2,
        WeaponClass::Sniper => 0.32,
    }
}

fn base_hip_fire_spread_deg(class: WeaponClass) -> f32 {
    match class {
        WeaponClass::Rifle => 4.8,
        WeaponClass::Smg => 3.8,
        WeaponClass::Sniper => 6.5,
    }
}

fn base_penetration_power(class: WeaponClass) -> f32 {
    match class {
        WeaponClass::Rifle => 1.0,
        WeaponClass::Smg => 0.8,
        WeaponClass::Sniper => 1.35,
    }
}

fn is_snake_case_id(id: &str) -> bool {
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

#[cfg(test)]
mod tests {
    use super::{attachment_ids_for_category, supports_attachment, supports_attachment_category};
    use crate::entity::weapon::WeaponId;
    use crate::entity::weapon::attachment::{AttachmentCategory, AttachmentId};

    fn weapon(name: &str) -> WeaponId {
        WeaponId::from_name(name).unwrap_or_else(|| panic!("missing weapon in catalog: {name}"))
    }

    #[test]
    fn rifles_support_underbarrel_but_snipers_do_not() {
        let ak = weapon("AK-47");
        let sniper = weapon("Sniper");
        assert!(supports_attachment_category(
            ak,
            AttachmentCategory::Underbarrel
        ));
        assert!(!supports_attachment_category(
            sniper,
            AttachmentCategory::Underbarrel
        ));
    }

    #[test]
    fn unsupported_category_is_rejected_for_smg() {
        let mp5 = weapon("MP5");
        assert!(!supports_attachment_category(mp5, AttachmentCategory::Grip));
        assert!(supports_attachment_category(
            mp5,
            AttachmentCategory::Magazine
        ));
    }

    #[test]
    fn weapon_attachment_ids_are_category_specific() {
        let ak = weapon("AK-47");
        let optics = attachment_ids_for_category(ak, AttachmentCategory::Optic);
        assert!(optics.iter().any(|id| id.id() == "red_dot"));
        assert!(optics.iter().any(|id| id.id() == "holo_sight"));
    }

    #[test]
    fn weapon_rejects_attachment_not_listed_in_json() {
        let sniper = weapon("Sniper");
        let laser = AttachmentId::new("laser_pointer", AttachmentCategory::Underbarrel);
        assert!(!supports_attachment(sniper, &laser));
    }
}
