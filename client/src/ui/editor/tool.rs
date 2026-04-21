use game::world::units::px_to_tiles;

// ---------------------------------------------------------------------------
// Editor constants
// ---------------------------------------------------------------------------

pub const BREAKABLE_HP: u32 = 1;
pub const TILE_GRID: f32 = 1.0;
pub const SUBGRID_DIVISIONS: usize = 8;
pub const EDGE_STEP: f32 = TILE_GRID / SUBGRID_DIVISIONS as f32;
pub const DEFAULT_ERASER_STEPS: u32 = 2;
pub const MAX_ERASER_STEPS: u32 = 16;

// ---------------------------------------------------------------------------
// Wall size (square blocks)
// ---------------------------------------------------------------------------

/// Discrete size steps for square wall blocks (in EDGE_STEP units).
/// 8 EDGE_STEPs = 1 tile, so these map to 1/8, 1/4, 1/2, 1 tile.
pub const WALL_STEP_VALUES: &[u32] = &[1, 2, 4, 8];
pub const DEFAULT_WALL_SIZE_STEPS: u32 = 8; // 1 tile

/// Cycle a step value through WALL_STEP_VALUES, wrapping around.
pub fn cycle_wall_step(current: u32) -> u32 {
    let idx = WALL_STEP_VALUES.iter().position(|&v| v == current).unwrap_or(0);
    WALL_STEP_VALUES[(idx + 1) % WALL_STEP_VALUES.len()]
}

pub fn step_label(steps: u32) -> &'static str {
    match steps {
        1 => "1/8",
        2 => "1/4",
        4 => "1/2",
        8 => "1",
        _ => "?",
    }
}

pub const GRID_LINE_THICKNESS: f32 = px_to_tiles(1.0);
pub const GRID_LINE_ALPHA: f32 = 0.10;
pub const SUBGRID_LINE_ALPHA: f32 = 0.05;
pub const EDITOR_MIN_ZOOM: f32 = 0.35;
pub const EDITOR_MAX_ZOOM: f32 = 3.5;
pub const EDITOR_KEY_ZOOM_STEP: f32 = 1.04;
pub const EDITOR_WHEEL_ZOOM_STEP: f32 = 0.12;
pub const EDITOR_PAN_STEP_PX: f32 = 20.0;

// ---------------------------------------------------------------------------
// Tool
// ---------------------------------------------------------------------------

#[derive(Default, Clone, Copy, PartialEq)]
pub enum Tool {
    #[default]
    PlayerSpawn, // key 1 — green
    Enemy,       // key 2 — red
    Wall,        // key 3 — brown, square block
    TargetDummy, // key 4 — yellow
    Breakable,   // key 5 — cyan, square breakable block
    Prop,        // key 6 — id-backed prop
    BaseMap,     // key 7 — 2-point map bounds rectangle
    Team1Spawn,  // key 8 — blue
    Team2Spawn,  // key 9 — orange
    Floor,       // key 0 — id-backed floor tile
    Eraser,      // key F — erase anything in area
}

// ---------------------------------------------------------------------------
// Snap mode
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
pub enum SnapMode {
    Edge,
    Center,
    Subgrid,
}

impl SnapMode {
    pub fn toggle(self) -> Self {
        match self {
            SnapMode::Edge => SnapMode::Center,
            SnapMode::Center => SnapMode::Subgrid,
            SnapMode::Subgrid => SnapMode::Edge,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SnapMode::Edge => "Tile Edges",
            SnapMode::Center => "Tile Centers",
            SnapMode::Subgrid => "Tile Subgrid (1/8)",
        }
    }
}
