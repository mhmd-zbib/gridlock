use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use super::WeaponStats;

pub const ATTACHMENT_CATEGORY_COUNT: usize = 6;
pub const ALL_ATTACHMENT_CATEGORIES: [AttachmentCategory; ATTACHMENT_CATEGORY_COUNT] = [
    AttachmentCategory::Optic,
    AttachmentCategory::Barrel,
    AttachmentCategory::Grip,
    AttachmentCategory::Magazine,
    AttachmentCategory::Stock,
    AttachmentCategory::Underbarrel,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttachmentCategory {
    Optic,
    Barrel,
    Grip,
    Magazine,
    Stock,
    Underbarrel,
}

impl AttachmentCategory {
    pub fn all() -> &'static [AttachmentCategory; ATTACHMENT_CATEGORY_COUNT] {
        &ALL_ATTACHMENT_CATEGORIES
    }

    pub fn label(self) -> &'static str {
        match self {
            AttachmentCategory::Optic => "Optic",
            AttachmentCategory::Barrel => "Barrel",
            AttachmentCategory::Grip => "Grip",
            AttachmentCategory::Magazine => "Magazine",
            AttachmentCategory::Stock => "Stock",
            AttachmentCategory::Underbarrel => "Underbarrel",
        }
    }

    pub(crate) fn index(self) -> usize {
        match self {
            AttachmentCategory::Optic => 0,
            AttachmentCategory::Barrel => 1,
            AttachmentCategory::Grip => 2,
            AttachmentCategory::Magazine => 3,
            AttachmentCategory::Stock => 4,
            AttachmentCategory::Underbarrel => 5,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AttachmentDef {
    pub id: &'static str,
    pub name: &'static str,
    pub category: AttachmentCategory,
    pub effects: AttachmentEffects,
}

#[derive(Deserialize)]
struct AttachmentDefJson {
    id: String,
    category: AttachmentCategory,
    name: String,
    #[serde(default)]
    effects: AttachmentEffects,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RecoilEffects {
    #[serde(default = "one")]
    pub vertical_recoil_mult: f32,
    #[serde(default = "one")]
    pub horizontal_recoil_mult: f32,
    #[serde(default = "one")]
    pub recoil_recovery_mult: f32,
}

impl Default for RecoilEffects {
    fn default() -> Self {
        Self {
            vertical_recoil_mult: 1.0,
            horizontal_recoil_mult: 1.0,
            recoil_recovery_mult: 1.0,
        }
    }
}

impl RecoilEffects {
    fn combine(self, other: Self) -> Self {
        Self {
            vertical_recoil_mult: self.vertical_recoil_mult * other.vertical_recoil_mult,
            horizontal_recoil_mult: self.horizontal_recoil_mult * other.horizontal_recoil_mult,
            recoil_recovery_mult: self.recoil_recovery_mult * other.recoil_recovery_mult,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct HandlingEffects {
    #[serde(default = "one")]
    pub ads_time_mult: f32,
    #[serde(default = "one")]
    pub weapon_swap_time_mult: f32,
    #[serde(default = "one")]
    pub reload_time_mult: f32,
    #[serde(default = "one")]
    pub sprint_to_fire_time_mult: f32,
    #[serde(default = "one")]
    pub mag_size_mult: f32,
}

impl Default for HandlingEffects {
    fn default() -> Self {
        Self {
            ads_time_mult: 1.0,
            weapon_swap_time_mult: 1.0,
            reload_time_mult: 1.0,
            sprint_to_fire_time_mult: 1.0,
            mag_size_mult: 1.0,
        }
    }
}

impl HandlingEffects {
    fn combine(self, other: Self) -> Self {
        Self {
            ads_time_mult: self.ads_time_mult * other.ads_time_mult,
            weapon_swap_time_mult: self.weapon_swap_time_mult * other.weapon_swap_time_mult,
            reload_time_mult: self.reload_time_mult * other.reload_time_mult,
            sprint_to_fire_time_mult: self.sprint_to_fire_time_mult
                * other.sprint_to_fire_time_mult,
            mag_size_mult: self.mag_size_mult * other.mag_size_mult,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AccuracyEffects {
    #[serde(default = "one")]
    pub movement_spread_mult: f32,
    #[serde(default = "one")]
    pub hip_fire_spread_mult: f32,
    #[serde(default = "one")]
    pub first_shot_spread_mult: f32,
}

impl Default for AccuracyEffects {
    fn default() -> Self {
        Self {
            movement_spread_mult: 1.0,
            hip_fire_spread_mult: 1.0,
            first_shot_spread_mult: 1.0,
        }
    }
}

impl AccuracyEffects {
    fn combine(self, other: Self) -> Self {
        Self {
            movement_spread_mult: self.movement_spread_mult * other.movement_spread_mult,
            hip_fire_spread_mult: self.hip_fire_spread_mult * other.hip_fire_spread_mult,
            first_shot_spread_mult: self.first_shot_spread_mult * other.first_shot_spread_mult,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PowerEffects {
    #[serde(default = "one")]
    pub damage_falloff_range_mult: f32,
    #[serde(default = "one")]
    pub bullet_velocity_mult: f32,
    #[serde(default = "one")]
    pub penetration_power_mult: f32,
    #[serde(default = "one")]
    pub bullet_damage_mult: f32,
}

impl Default for PowerEffects {
    fn default() -> Self {
        Self {
            damage_falloff_range_mult: 1.0,
            bullet_velocity_mult: 1.0,
            penetration_power_mult: 1.0,
            bullet_damage_mult: 1.0,
        }
    }
}

impl PowerEffects {
    fn combine(self, other: Self) -> Self {
        Self {
            damage_falloff_range_mult: self.damage_falloff_range_mult
                * other.damage_falloff_range_mult,
            bullet_velocity_mult: self.bullet_velocity_mult * other.bullet_velocity_mult,
            penetration_power_mult: self.penetration_power_mult * other.penetration_power_mult,
            bullet_damage_mult: self.bullet_damage_mult * other.bullet_damage_mult,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct StealthEffects {
    #[serde(default = "one")]
    pub shot_sound_radius_mult: f32,
    #[serde(default = "one")]
    pub minimap_signature_mult: f32,
    #[serde(default = "one")]
    pub muzzle_flash_mult: f32,
}

impl Default for StealthEffects {
    fn default() -> Self {
        Self {
            shot_sound_radius_mult: 1.0,
            minimap_signature_mult: 1.0,
            muzzle_flash_mult: 1.0,
        }
    }
}

impl StealthEffects {
    fn combine(self, other: Self) -> Self {
        Self {
            shot_sound_radius_mult: self.shot_sound_radius_mult * other.shot_sound_radius_mult,
            minimap_signature_mult: self.minimap_signature_mult * other.minimap_signature_mult,
            muzzle_flash_mult: self.muzzle_flash_mult * other.muzzle_flash_mult,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MobilityEffects {
    #[serde(default = "one")]
    pub ads_move_speed_mult: f32,
    #[serde(default = "one")]
    pub strafe_spread_penalty_mult: f32,
    #[serde(default = "one")]
    pub jump_stability_mult: f32,
}

impl Default for MobilityEffects {
    fn default() -> Self {
        Self {
            ads_move_speed_mult: 1.0,
            strafe_spread_penalty_mult: 1.0,
            jump_stability_mult: 1.0,
        }
    }
}

impl MobilityEffects {
    fn combine(self, other: Self) -> Self {
        Self {
            ads_move_speed_mult: self.ads_move_speed_mult * other.ads_move_speed_mult,
            strafe_spread_penalty_mult: self.strafe_spread_penalty_mult
                * other.strafe_spread_penalty_mult,
            jump_stability_mult: self.jump_stability_mult * other.jump_stability_mult,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct UtilityEffects {
    #[serde(default = "one")]
    pub ping_visibility_mult: f32,
    #[serde(default = "one")]
    pub detection_strength_mult: f32,
    #[serde(default = "one")]
    pub enemy_mark_duration_mult: f32,
}

impl Default for UtilityEffects {
    fn default() -> Self {
        Self {
            ping_visibility_mult: 1.0,
            detection_strength_mult: 1.0,
            enemy_mark_duration_mult: 1.0,
        }
    }
}

impl UtilityEffects {
    fn combine(self, other: Self) -> Self {
        Self {
            ping_visibility_mult: self.ping_visibility_mult * other.ping_visibility_mult,
            detection_strength_mult: self.detection_strength_mult * other.detection_strength_mult,
            enemy_mark_duration_mult: self.enemy_mark_duration_mult
                * other.enemy_mark_duration_mult,
        }
    }
}

fn one() -> f32 {
    1.0
}

fn effects_are_valid(effects: AttachmentEffects) -> bool {
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

static CATALOG: OnceLock<Vec<AttachmentDef>> = OnceLock::new();

pub fn all_attachment_defs() -> &'static [AttachmentDef] {
    catalog()
}

pub fn all_attachment_ids() -> Vec<AttachmentId> {
    catalog()
        .iter()
        .map(|entry| AttachmentId::new(entry.id, entry.category))
        .collect()
}

pub fn attachment_ids_for_category(category: AttachmentCategory) -> Vec<AttachmentId> {
    catalog()
        .iter()
        .filter_map(|entry| {
            (entry.category == category).then_some(AttachmentId::new(entry.id, entry.category))
        })
        .collect()
}

pub fn attachment_id_by_name(name: &str) -> Option<AttachmentId> {
    catalog()
        .iter()
        .find(|entry| entry.name == name || entry.id.eq_ignore_ascii_case(name))
        .map(|entry| AttachmentId::new(entry.id, entry.category))
}

pub fn attachment_id_by_id(id: &str) -> Option<AttachmentId> {
    catalog()
        .iter()
        .find(|entry| entry.id.eq_ignore_ascii_case(id))
        .map(|entry| AttachmentId::new(entry.id, entry.category))
}

pub fn attachment_name(id: &AttachmentId) -> Option<&'static str> {
    catalog()
        .iter()
        .find(|entry| entry.id == id.id() && entry.category == id.category())
        .map(|entry| entry.name)
}

pub fn attachment_effects(id: &AttachmentId) -> Option<AttachmentEffects> {
    catalog()
        .iter()
        .find(|entry| entry.id == id.id() && entry.category == id.category())
        .map(|entry| entry.effects)
}

fn catalog() -> &'static [AttachmentDef] {
    CATALOG.get_or_init(load_catalog).as_slice()
}

fn load_catalog() -> Vec<AttachmentDef> {
    let attachments_dir = resolve_attachments_dir();
    let read_dir = fs::read_dir(&attachments_dir).unwrap_or_else(|e| {
        panic!(
            "failed to read attachments directory '{}': {e}",
            attachments_dir.display()
        )
    });

    let mut entries: Vec<AttachmentDef> = read_dir
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .map(|path| load_attachment_file(&path))
        .collect();

    if entries.is_empty() {
        panic!(
            "no attachment JSON files found in attachments directory '{}'",
            attachments_dir.display()
        );
    }

    entries.sort_by(|a, b| {
        a.category
            .index()
            .cmp(&b.category.index())
            .then_with(|| a.name.cmp(b.name))
    });

    let mut seen_ids = HashSet::new();
    for entry in &entries {
        let normalized = entry.id.to_ascii_lowercase();
        assert!(
            seen_ids.insert(normalized),
            "duplicate attachment id '{}'",
            entry.id
        );
    }

    entries
}

fn resolve_attachments_dir() -> PathBuf {
    let cwd_attachments = std::env::current_dir()
        .ok()
        .map(|cwd| cwd.join("attachments"))
        .filter(|path| path.is_dir());
    if let Some(path) = cwd_attachments {
        return path;
    }

    let manifest_attachments = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("attachments");
    if manifest_attachments.is_dir() {
        return manifest_attachments;
    }

    panic!(
        "attachments directory not found (checked './attachments' and '{}/attachments')",
        env!("CARGO_MANIFEST_DIR")
    );
}

fn load_attachment_file(path: &Path) -> AttachmentDef {
    let json = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read attachment file '{}': {e}", path.display()));
    let parsed: AttachmentDefJson = serde_json::from_str(&json)
        .unwrap_or_else(|e| panic!("invalid attachment json '{}': {e}", path.display()));

    if parsed.name.trim().is_empty() {
        panic!("attachment file '{}' has empty 'name'", path.display());
    }
    if !is_snake_case_id(&parsed.id) {
        panic!(
            "attachment file '{}' has non-snake-case id '{}'",
            path.display(),
            parsed.id
        );
    }
    if !effects_are_valid(parsed.effects) {
        panic!(
            "attachment file '{}' has invalid effect multipliers",
            path.display()
        );
    }

    AttachmentDef {
        id: leak_string(parsed.id),
        name: leak_string(parsed.name),
        category: parsed.category,
        effects: parsed.effects,
    }
}

fn leak_string(value: String) -> &'static str {
    Box::leak(value.into_boxed_str())
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

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AttachmentId {
    id: String,
    category: AttachmentCategory,
}

impl AttachmentId {
    pub fn new(id: impl Into<String>, category: AttachmentCategory) -> Self {
        Self {
            id: id.into(),
            category,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn category(&self) -> AttachmentCategory {
        self.category
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AttachmentEquipError {
    CategoryMismatch {
        slot: AttachmentCategory,
        attachment: AttachmentCategory,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentLoadout {
    slots: [Option<AttachmentId>; ATTACHMENT_CATEGORY_COUNT],
}

impl Default for AttachmentLoadout {
    fn default() -> Self {
        Self {
            slots: std::array::from_fn(|_| None),
        }
    }
}

impl AttachmentLoadout {
    pub fn is_empty(&self, category: AttachmentCategory) -> bool {
        self.get(category).is_none()
    }

    pub fn get(&self, category: AttachmentCategory) -> Option<&AttachmentId> {
        self.slots[category.index()].as_ref()
    }

    pub fn unequip(&mut self, category: AttachmentCategory) -> Option<AttachmentId> {
        self.slots[category.index()].take()
    }

    pub fn equip(&mut self, attachment: AttachmentId) -> Option<AttachmentId> {
        self.replace(attachment.category(), attachment)
    }

    pub fn equip_in_slot(
        &mut self,
        slot: AttachmentCategory,
        attachment: AttachmentId,
    ) -> Result<Option<AttachmentId>, AttachmentEquipError> {
        if slot != attachment.category() {
            return Err(AttachmentEquipError::CategoryMismatch {
                slot,
                attachment: attachment.category(),
            });
        }
        Ok(self.replace(slot, attachment))
    }

    fn replace(
        &mut self,
        slot: AttachmentCategory,
        attachment: AttachmentId,
    ) -> Option<AttachmentId> {
        let idx = slot.index();
        let mut replaced = Some(attachment);
        std::mem::swap(&mut self.slots[idx], &mut replaced);
        replaced
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ALL_ATTACHMENT_CATEGORIES, AccuracyEffects, AttachmentCategory, AttachmentEffects,
        AttachmentEquipError, AttachmentId, AttachmentLoadout, all_attachment_defs,
        all_attachment_ids, attachment_effects, attachment_id_by_id, attachment_id_by_name,
        attachment_ids_for_category, attachment_name,
    };

    #[test]
    fn attachment_categories_are_fixed_and_complete() {
        assert_eq!(
            ALL_ATTACHMENT_CATEGORIES,
            [
                AttachmentCategory::Optic,
                AttachmentCategory::Barrel,
                AttachmentCategory::Grip,
                AttachmentCategory::Magazine,
                AttachmentCategory::Stock,
                AttachmentCategory::Underbarrel,
            ]
        );
    }

    #[test]
    fn attachment_catalog_loads_from_json() {
        let defs = all_attachment_defs();
        let ids = all_attachment_ids();
        assert!(!defs.is_empty());
        assert_eq!(defs.len(), ids.len());
        assert!(defs.iter().any(|def| {
            def.id == "red_dot"
                && def.category == AttachmentCategory::Optic
                && def.name == "Red Dot"
        }));
    }

    #[test]
    fn attachment_lookup_by_name_and_id_is_case_flexible() {
        let from_name = attachment_id_by_name("Red Dot").expect("Red Dot should exist");
        let from_id = attachment_id_by_name("RED_DOT").expect("red_dot should exist");
        let by_id = attachment_id_by_id("ReD_DoT").expect("red_dot should exist");
        assert_eq!(from_name.id(), "red_dot");
        assert_eq!(from_name.category(), AttachmentCategory::Optic);
        assert_eq!(from_id.id(), "red_dot");
        assert_eq!(by_id.id(), "red_dot");
        assert_eq!(attachment_name(&from_id), Some("Red Dot"));
    }

    #[test]
    fn each_fixed_category_has_at_least_one_catalog_entry() {
        for category in AttachmentCategory::all() {
            assert!(
                !attachment_ids_for_category(*category).is_empty(),
                "missing attachment for category {category:?}"
            );
        }
    }

    #[test]
    fn effects_can_be_read_for_attachment() {
        let laser = AttachmentId::new("laser_pointer", AttachmentCategory::Underbarrel);
        let fx = attachment_effects(&laser).expect("laser_pointer should have effects");
        assert!(fx.accuracy.hip_fire_spread_mult < 1.0);
        assert!(fx.utility.ping_visibility_mult > 1.0);
    }

    #[test]
    fn effect_combine_multiplies_values() {
        let a = AttachmentEffects {
            accuracy: AccuracyEffects {
                hip_fire_spread_mult: 0.9,
                ..Default::default()
            },
            ..Default::default()
        };
        let b = AttachmentEffects {
            accuracy: AccuracyEffects {
                hip_fire_spread_mult: 0.8,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = a.combine(b);
        assert!((out.accuracy.hip_fire_spread_mult - 0.72).abs() < 0.0001);
    }

    #[test]
    fn empty_slot_is_none_and_equip_sets_one_active_attachment() {
        let mut loadout = AttachmentLoadout::default();
        assert!(loadout.is_empty(AttachmentCategory::Optic));

        let optic = AttachmentId::new("red_dot", AttachmentCategory::Optic);
        assert_eq!(loadout.equip(optic), None);
        assert_eq!(
            loadout.get(AttachmentCategory::Optic).map(|a| a.id()),
            Some("red_dot")
        );
    }

    #[test]
    fn replacing_attachment_overrides_previous_one_in_same_slot() {
        let mut loadout = AttachmentLoadout::default();

        let first = AttachmentId::new("red_dot", AttachmentCategory::Optic);
        let second = AttachmentId::new("holo_sight", AttachmentCategory::Optic);

        assert_eq!(loadout.equip(first), None);
        let replaced = loadout.equip(second);

        assert_eq!(
            replaced.map(|a| a.id().to_string()),
            Some("red_dot".to_string())
        );
        assert_eq!(
            loadout.get(AttachmentCategory::Optic).map(|a| a.id()),
            Some("holo_sight")
        );
    }

    #[test]
    fn cross_category_equip_is_rejected() {
        let mut loadout = AttachmentLoadout::default();
        let attachment = AttachmentId::new("red_dot", AttachmentCategory::Optic);

        let err = loadout
            .equip_in_slot(AttachmentCategory::Barrel, attachment)
            .expect_err("category mismatch should fail");

        assert_eq!(
            err,
            AttachmentEquipError::CategoryMismatch {
                slot: AttachmentCategory::Barrel,
                attachment: AttachmentCategory::Optic,
            }
        );
    }
}
