use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// A floor tile placement stored inside level JSON.
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct LevelFloor {
    pub x: f32,
    pub y: f32,
    pub id: String,
}

/// One placeable floor definition loaded from an asset JSON file.
#[derive(Serialize, Deserialize, Clone)]
pub struct FloorAssetDef {
    pub id: String,
    #[serde(default = "default_one")]
    pub width: f32,
    #[serde(default = "default_one")]
    pub height: f32,
    #[serde(default)]
    pub texture: Option<String>,
}

fn default_one() -> f32 {
    1.0
}

#[derive(Clone)]
pub struct ResolvedFloor {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub id: String,
}

pub fn load_assets() -> Vec<FloorAssetDef> {
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

pub fn resolve_level_floors(
    level_floors: &[LevelFloor],
    assets: &[FloorAssetDef],
) -> Vec<ResolvedFloor> {
    let by_id: HashMap<&str, &FloorAssetDef> =
        assets.iter().map(|def| (def.id.as_str(), def)).collect();
    let mut out = Vec::with_capacity(level_floors.len());

    for floor in level_floors {
        let Some(def) = by_id.get(floor.id.as_str()) else {
            continue;
        };
        out.push(ResolvedFloor {
            x: floor.x,
            y: floor.y,
            width: def.width,
            height: def.height,
            id: def.id.clone(),
        });
    }

    out
}

pub fn asset_color(id: &str, alpha: f32) -> [f32; 4] {
    let mut hash: u32 = 0x811C9DC5;
    for b in id.bytes() {
        hash ^= b as u32;
        hash = hash.wrapping_mul(0x01000193);
    }

    // Muted, desaturated earth tones so floors stay visually beneath props/walls.
    let r = 0.18 + ((hash & 0xFF) as f32 / 255.0) * 0.22;
    let g = 0.16 + (((hash >> 8) & 0xFF) as f32 / 255.0) * 0.20;
    let b = 0.12 + (((hash >> 16) & 0xFF) as f32 / 255.0) * 0.16;

    [r, g, b, alpha]
}

fn resolve_assets_dir() -> Option<PathBuf> {
    let cwd_assets = std::env::current_dir()
        .ok()
        .map(|cwd| cwd.join("assets").join("configs").join("floors"))
        .filter(|path| path.is_dir());
    if let Some(path) = cwd_assets {
        return Some(path);
    }

    let manifest_assets = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("configs")
        .join("floors");
    if manifest_assets.is_dir() {
        return Some(manifest_assets);
    }

    let workspace_assets = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|root| root.join("assets").join("configs").join("floors"))
        .filter(|path| path.is_dir());
    if let Some(path) = workspace_assets {
        return Some(path);
    }

    None
}

fn load_asset_file(path: &Path) -> Option<FloorAssetDef> {
    let Ok(json) = fs::read_to_string(path) else {
        return None;
    };
    let Ok(parsed) = serde_json::from_str::<FloorAssetDef>(&json) else {
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
