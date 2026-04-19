use serde::{Deserialize, Serialize};

use super::{floor::LevelFloor, prop::LevelProp, wall::Wall};

/// A 2-D position stored in the level file.
#[derive(Serialize, Deserialize, Clone, Copy, Default)]
pub struct Pos {
    pub x: f32,
    pub y: f32,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct LevelBounds {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Everything the game needs to know to set up one level.
/// Serialises to / deserialises from a single JSON file.
///
/// Example file:
/// ```json
/// {
///   "id": "level_01",
///   "map_bounds": { "x": 0.0, "y": 0.0, "w": 24.0, "h": 14.0 },
///   "player_spawn": { "x": 6.25, "y": 4.6875 },
///   "enemies": [
///     { "x": 1.5625, "y": 1.5625 },
///     { "x": 10.9375, "y": 3.125 }
///   ],
///   "target_enemies": [
///     { "x": 7.8125, "y": 4.6875 }
///   ],
///   "props": [
///     { "x": 5.0, "y": 4.0, "id": "crate_01" }
///   ]
/// }
/// ```
#[derive(Serialize, Deserialize, Clone)]
pub struct LevelData {
    pub id: String,
    #[serde(default)]
    pub map_bounds: Option<LevelBounds>,
    /// `None` means no spawn defined yet (editor hasn't placed one).
    pub player_spawn: Option<Pos>,
    #[serde(default)]
    pub team1_spawn: Option<Pos>,
    #[serde(default)]
    pub team2_spawn: Option<Pos>,
    #[serde(default)]
    pub enemies: Vec<Pos>,
    #[serde(default)]
    pub target_enemies: Vec<Pos>,
    #[serde(default)]
    pub walls: Vec<Wall>,
    #[serde(default)]
    pub floors: Vec<LevelFloor>,
    #[serde(default)]
    pub props: Vec<LevelProp>,
}

impl Default for LevelData {
    fn default() -> Self {
        Self {
            id: "level_2".to_string(),
            map_bounds: None,
            player_spawn: None,
            team1_spawn: None,
            team2_spawn: None,
            enemies: Vec::new(),
            target_enemies: Vec::new(),
            walls: Vec::new(),
            floors: Vec::new(),
            props: Vec::new(),
        }
    }
}

impl LevelData {
    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let text = std::fs::read_to_string(path)?;
        let data: Self = serde_json::from_str(&text)?;
        validate_level_data(&data)?;
        Ok(data)
    }

    pub fn save(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        validate_level_data(self)?;
        if let Some(dir) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }
}

fn validate_level_data(level: &LevelData) -> Result<(), Box<dyn std::error::Error>> {
    if !is_snake_case_id(&level.id) {
        let err = std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("level id '{}' must be snake_case", level.id),
        );
        return Err(Box::new(err));
    }
    for floor in &level.floors {
        if !is_snake_case_id(&floor.id) {
            let err = std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("level floor id '{}' must be snake_case", floor.id),
            );
            return Err(Box::new(err));
        }
    }
    for prop in &level.props {
        if !is_snake_case_id(&prop.id) {
            let err = std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("level prop id '{}' must be snake_case", prop.id),
            );
            return Err(Box::new(err));
        }
    }
    if let Some(bounds) = level.map_bounds {
        if bounds.w <= 0.0 || bounds.h <= 0.0 {
            let err = std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "map_bounds must have positive size, got w={} h={}",
                    bounds.w, bounds.h
                ),
            );
            return Err(Box::new(err));
        }
    }
    Ok(())
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
