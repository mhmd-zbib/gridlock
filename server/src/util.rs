use game::entity::weapon::{WeaponState, all_weapon_ids};
use game::world::level::LevelData;
use game::world::units::px_to_tiles;

/// Return the default weapon given to every newly connected player.
pub fn default_weapon_state() -> WeaponState {
    let default_weapon = all_weapon_ids()
        .first()
        .copied()
        .expect("weapon catalog is empty");
    WeaponState::new(default_weapon)
}

/// Compute a small circular spawn offset for player `player_id` so that
/// freshly connected players do not all pile up on exactly the same point.
pub fn spawn_offset(player_id: u16) -> (f32, f32) {
    if player_id <= 1 {
        return (0.0, 0.0);
    }
    let idx = (player_id - 1) as f32;
    let angle = idx * 2.399_963_2; // ≈ golden-angle step in radians
    let radius = px_to_tiles(26.0);
    (angle.cos() * radius, angle.sin() * radius)
}

/// Current wall-clock time in milliseconds, truncated to u32.
pub fn server_time_ms() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u32
}

/// Load the first level file found under `assets/levels/` (sorted
/// alphabetically) so the server starts with the same walls and player spawn
/// point as the client.
pub fn load_first_level() -> Option<LevelData> {
    let dir = std::fs::read_dir("assets/levels").ok()?;
    let mut paths: Vec<_> = dir
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("json"))
        .collect();
    paths.sort();
    paths
        .first()
        .and_then(|p| p.to_str())
        .and_then(|s| LevelData::load(s).ok())
}
