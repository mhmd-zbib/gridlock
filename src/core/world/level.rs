use serde::{Deserialize, Serialize};

use super::wall::Wall;

/// A 2-D position stored in the level file.
#[derive(Serialize, Deserialize, Clone, Copy, Default)]
pub struct Pos {
    pub x: f32,
    pub y: f32,
}

/// Everything the game needs to know to set up one level.
/// Serialises to / deserialises from a single JSON file.
///
/// Example file:
/// ```json
/// {
///   "name": "level_01",
///   "player_spawn": { "x": 6.25, "y": 4.6875 },
///   "enemies": [
///     { "x": 1.5625, "y": 1.5625 },
///     { "x": 10.9375, "y": 3.125 }
///   ],
///   "target_enemies": [
///     { "x": 7.8125, "y": 4.6875 }
///   ]
/// }
/// ```
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct LevelData {
    pub name: String,
    /// `None` means no spawn defined yet (editor hasn't placed one).
    pub player_spawn: Option<Pos>,
    #[serde(default)]
    pub enemies: Vec<Pos>,
    #[serde(default)]
    pub target_enemies: Vec<Pos>,
    #[serde(default)]
    pub walls: Vec<Wall>,
}

impl LevelData {
    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let text = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&text)?)
    }

    pub fn save(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(dir) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }
}
