use game::entity::enemy::{Enemy, EnemyKind};
use game::game::Game;
use game::world::camera::CameraBounds;
use game::world::level::LevelData;
use game::world::units::px_to_tiles;

/// Returns `true` if any Shooter enemy is currently in full combat state.
/// Used by the camera system to switch between exploration and combat modes.
pub fn enemies_in_combat(enemies: &[Enemy]) -> bool {
    enemies
        .iter()
        .any(|e| e.kind == EnemyKind::Shooter && e.brain.awareness.in_combat())
}

/// Derive camera bounds from level data.
///
/// If the level has authored map bounds they are used with a generous padding
/// so edges do not feel like hard sticky walls.  Otherwise the bounds are
/// inferred from all entity and wall positions.
pub fn infer_world_bounds(game: &Game, fallback_w: f32, fallback_h: f32) -> CameraBounds {
    if let Some(bounds) = game.level_bounds {
        const MAP_BOUNDS_CAMERA_PADDING: f32 = px_to_tiles(256.0);
        return CameraBounds::from_min_max(
            bounds.x - MAP_BOUNDS_CAMERA_PADDING,
            bounds.y - MAP_BOUNDS_CAMERA_PADDING,
            bounds.x + bounds.w + MAP_BOUNDS_CAMERA_PADDING,
            bounds.y + bounds.h + MAP_BOUNDS_CAMERA_PADDING,
        );
    }

    const EDGE_PADDING: f32 = px_to_tiles(128.0);
    let mut min_x = 0.0_f32;
    let mut min_y = 0.0_f32;
    let mut max_x = fallback_w.max(game.player.movement.x);
    let mut max_y = fallback_h.max(game.player.movement.y);

    let mut include = |x: f32, y: f32| {
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    };

    include(game.player.movement.x, game.player.movement.y);
    for enemy in &game.enemies {
        include(enemy.movement.x, enemy.movement.y);
    }
    for wall in &game.walls {
        include(wall.x, wall.y);
        include(wall.x + wall.w, wall.y + wall.h);
    }
    for prop in &game.props {
        include(prop.x - prop.width * 0.5, prop.y - prop.height * 0.5);
        include(prop.x + prop.width * 0.5, prop.y + prop.height * 0.5);
    }

    min_x = (min_x - EDGE_PADDING).min(0.0);
    min_y = (min_y - EDGE_PADDING).min(0.0);
    max_x = (max_x + EDGE_PADDING).max(fallback_w);
    max_y = (max_y + EDGE_PADDING).max(fallback_h);
    CameraBounds::from_min_max(min_x, min_y, max_x, max_y)
}

/// Current wall-clock time in milliseconds, truncated to u32.
/// Used to timestamp outgoing ClientPacket messages.
pub fn client_time_ms() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u32
}

/// Scan `assets/levels/` and load the first JSON file (sorted alphabetically).
/// Mirrors the server-side `load_first_level` so both start with the same level.
pub fn scan_first_level() -> Option<LevelData> {
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
