use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::Deserialize;

use super::category::AttachmentCategory;
use super::effects::{AttachmentEffects, effects_are_valid};
use super::loadout::AttachmentId;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AttachmentDef {
    pub id: &'static str,
    pub name: &'static str,
    pub category: AttachmentCategory,
    pub effects: AttachmentEffects,
}

#[derive(Deserialize)]
pub(super) struct AttachmentDefJson {
    pub id: String,
    pub category: AttachmentCategory,
    pub name: String,
    #[serde(default)]
    pub effects: AttachmentEffects,
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
        .map(|cwd| cwd.join("assets").join("attachments"))
        .filter(|path| path.is_dir());
    if let Some(path) = cwd_attachments {
        return path;
    }

    let manifest_attachments = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("attachments");
    if manifest_attachments.is_dir() {
        return manifest_attachments;
    }

    let workspace_attachments = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|root| root.join("assets").join("attachments"));
    if let Some(path) = workspace_attachments.as_ref().filter(|path| path.is_dir()) {
        return path.to_path_buf();
    }

    panic!(
        "attachments directory not found (checked './assets/attachments', '{}/assets/attachments', and '{}')",
        env!("CARGO_MANIFEST_DIR"),
        workspace_attachments
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "<workspace-root-unavailable>/assets/attachments".to_string())
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
