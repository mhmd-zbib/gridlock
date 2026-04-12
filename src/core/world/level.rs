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
///   "player_spawn": { "x": 400.0, "y": 300.0 },
///   "enemies": [
///     { "x": 100.0, "y": 100.0 },
///     { "x": 700.0, "y": 200.0 }
///   ]
/// }
/// ```
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct LevelData {
    pub name: String,
    /// `None` means no spawn defined yet (editor hasn't placed one).
    pub player_spawn: Option<Pos>,
    pub enemies: Vec<Pos>,
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
