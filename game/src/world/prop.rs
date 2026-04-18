use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// A prop placement stored inside level JSON.
/// `id` must match one entry from `assets/props/*.json`.
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct LevelProp {
    pub x: f32,
    pub y: f32,
    #[serde(alias = "asset")]
    pub id: String,
}

/// One placeable prop definition loaded from an asset JSON file.
#[derive(Serialize, Deserialize, Clone)]
pub struct PropAssetDef {
    #[serde(alias = "asset")]
    pub id: String,
    pub width: f32,
    pub height: f32,
    #[serde(default, alias = "is collider", alias = "isCollider")]
    pub is_collider: bool,
}

#[derive(Clone)]
pub struct ResolvedProp {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub id: String,
    pub is_collider: bool,
}

pub fn load_assets() -> Vec<PropAssetDef> {
    let Some(dir) = resolve_assets_dir() else {
        return Vec::new();
    };
    let Ok(entries) = fs::read_dir(&dir) else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for path in entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
    {
        if let Some(def) = load_asset_file(&path) {
            out.push(def);
        }
    }

    out.sort_by(|a, b| a.id.cmp(&b.id));

    let mut seen = HashSet::new();
    out.retain(|def| seen.insert(def.id.clone()));
    out
}

pub fn resolve_level_props(
    level_props: &[LevelProp],
    assets: &[PropAssetDef],
) -> Vec<ResolvedProp> {
    let by_id: HashMap<&str, &PropAssetDef> =
        assets.iter().map(|def| (def.id.as_str(), def)).collect();
    let mut out = Vec::with_capacity(level_props.len());

    for prop in level_props {
        let Some(def) = by_id.get(prop.id.as_str()) else {
            continue;
        };
        out.push(ResolvedProp {
            x: prop.x,
            y: prop.y,
            width: def.width,
            height: def.height,
            id: def.id.clone(),
            is_collider: def.is_collider,
        });
    }

    out
}

pub fn asset_color(id: &str, is_collider: bool, alpha: f32) -> [f32; 4] {
    let mut hash: u32 = 0x811C9DC5;
    for b in id.bytes() {
        hash ^= b as u32;
        hash = hash.wrapping_mul(0x01000193);
    }

    let r = 0.25 + ((hash & 0xFF) as f32 / 255.0) * 0.55;
    let g = 0.25 + (((hash >> 8) & 0xFF) as f32 / 255.0) * 0.55;
    let b = 0.25 + (((hash >> 16) & 0xFF) as f32 / 255.0) * 0.55;

    if is_collider {
        [r, (g + 0.25).min(1.0), (b * 0.75).min(1.0), alpha]
    } else {
        [r, g, b, alpha]
    }
}

fn resolve_assets_dir() -> Option<PathBuf> {
    let cwd_assets = std::env::current_dir()
        .ok()
        .map(|cwd| cwd.join("assets").join("props"))
        .filter(|path| path.is_dir());
    if let Some(path) = cwd_assets {
        return Some(path);
    }

    let manifest_assets = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("props");
    if manifest_assets.is_dir() {
        return Some(manifest_assets);
    }

    let workspace_assets = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|root| root.join("assets").join("props"))
        .filter(|path| path.is_dir());
    if let Some(path) = workspace_assets {
        return Some(path);
    }

    None
}

fn load_asset_file(path: &Path) -> Option<PropAssetDef> {
    let Ok(json) = fs::read_to_string(path) else {
        return None;
    };
    let Ok(parsed) = serde_json::from_str::<PropAssetDef>(&json) else {
        return None;
    };

    if parsed.id.trim().is_empty() {
        return None;
    }
    if !is_snake_case_id(&parsed.id) {
        return None;
    }
    if parsed.width <= 0.0 || parsed.height <= 0.0 {
        return None;
    }
    Some(parsed)
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
