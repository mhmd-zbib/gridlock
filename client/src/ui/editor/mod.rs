mod assets;
mod helpers;
mod level_edit;
mod render;
mod tool;
mod update;
mod viewport;

pub use tool::{SnapMode, Tool, step_label};

use game::world::floor::{self as floor_world, FloorAssetDef};
use game::world::level::LevelData;
use game::world::prop::{self, PropAssetDef};

use helpers::effective_snap_mode;
use tool::{DEFAULT_ERASER_STEPS, DEFAULT_WALL_SIZE_STEPS, EDGE_STEP};

const EDITOR_LEVELS_DIR: &str = "assets/levels";
const LEGACY_EDITOR_LEVELS_DIR: &str = "levels";

pub struct Editor {
    pub tool: Tool,
    pub level: LevelData,
    pub snap_mode: SnapMode,
    pub show_subgrid: bool,
    pub show_textures: bool,
    pub zoom: f32,
    prop_assets: Vec<PropAssetDef>,
    selected_prop_asset: usize,
    floor_assets: Vec<FloorAssetDef>,
    selected_floor_asset: usize,
    view_origin: (f32, f32),
    pub wall_size_steps: u32,
    pub eraser_size_steps: u32,

    wall_drag_start: Option<(f32, f32)>,
    breakable_drag_start: Option<(f32, f32)>,
    base_map_start: Option<(f32, f32)>,
    floor_drag_last: Option<(f32, f32)>,
    right_drag_last: Option<(f32, f32)>,

    prev_left: bool,
    prev_right: bool,
    prev_f5: bool,
    prev_key_l: bool,
    prev_key_1: bool,
    prev_key_2: bool,
    prev_key_3: bool,
    prev_key_4: bool,
    prev_key_5: bool,
    prev_key_6: bool,
    prev_key_7: bool,
    prev_key_8: bool,
    prev_key_9: bool,
    prev_key_0: bool,
    prev_key_q: bool,
    prev_key_e: bool,
    prev_key_f: bool,
    prev_key_g: bool,
    prev_key_h: bool,
    prev_key_t: bool,
}

impl Editor {
    pub fn new() -> Self {
        let prop_assets = prop::load_assets();
        let floor_assets = floor_world::load_assets();
        Self {
            tool: Tool::default(),
            level: LevelData::default(),
            snap_mode: SnapMode::Subgrid,
            show_subgrid: true,
            show_textures: true,
            zoom: 1.0,
            prop_assets,
            selected_prop_asset: 0,
            floor_assets,
            selected_floor_asset: 0,
            view_origin: (0.0, 0.0),
            wall_size_steps: DEFAULT_WALL_SIZE_STEPS,
            eraser_size_steps: DEFAULT_ERASER_STEPS,
            wall_drag_start: None,
            breakable_drag_start: None,
            base_map_start: None,
            floor_drag_last: None,
            right_drag_last: None,
            prev_left: false,
            prev_right: false,
            prev_f5: false,
            prev_key_l: false,
            prev_key_1: false,
            prev_key_2: false,
            prev_key_3: false,
            prev_key_4: false,
            prev_key_5: false,
            prev_key_6: false,
            prev_key_7: false,
            prev_key_8: false,
            prev_key_9: false,
            prev_key_0: false,
            prev_key_q: false,
            prev_key_e: false,
            prev_key_f: false,
            prev_key_g: false,
            prev_key_h: false,
            prev_key_t: false,
        }
    }

    pub fn refresh_prop_assets(&mut self) {
        self.prop_assets = prop::load_assets();
        if self.selected_prop_asset >= self.prop_assets.len() {
            self.selected_prop_asset = 0;
        }
    }

    pub fn prop_assets(&self) -> &[PropAssetDef] {
        &self.prop_assets
    }

    pub fn selected_prop_asset(&self) -> Option<&PropAssetDef> {
        self.selected_prop_definition()
    }

    pub fn selected_prop_asset_index(&self) -> usize {
        self.selected_prop_asset
    }

    pub fn refresh_floor_assets(&mut self) {
        self.floor_assets = floor_world::load_assets();
        if self.selected_floor_asset >= self.floor_assets.len() {
            self.selected_floor_asset = 0;
        }
    }

    pub fn floor_assets(&self) -> &[FloorAssetDef] {
        &self.floor_assets
    }

    pub fn selected_floor_asset(&self) -> Option<&FloorAssetDef> {
        self.floor_assets.get(self.selected_floor_asset)
    }

    pub fn selected_floor_asset_index(&self) -> usize {
        self.selected_floor_asset
    }

    pub fn wall_block_size(&self) -> f32 {
        self.wall_size_steps as f32 * EDGE_STEP
    }

    pub fn active_snap_label(&self) -> &'static str {
        effective_snap_mode(self.tool, self.snap_mode).label()
    }

    fn level_file_name(&self) -> String {
        format!("{}.json", self.level.id)
    }

    fn active_level_path(&self) -> String {
        format!("{}/{}", EDITOR_LEVELS_DIR, self.level_file_name())
    }

    fn legacy_level_path(&self) -> String {
        format!("{}/{}", LEGACY_EDITOR_LEVELS_DIR, self.level_file_name())
    }

    fn apply_loaded_level(&mut self, data: LevelData) {
        self.level = data;
    }

    fn load_from_path(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let data = LevelData::load(path)?;
        self.apply_loaded_level(data);
        Ok(())
    }

    pub fn save_active_level(&self) {
        let path = self.active_level_path();
        match self.level.save(&path) {
            Ok(()) => {
                eprintln!("Editor: saved level to {}", path);
            }
            Err(err) => {
                eprintln!("Editor: failed to save level to {}: {}", path, err);
            }
        }
    }

    pub fn load_active_level(&mut self) {
        let path = self.active_level_path();
        if self.load_from_path(&path).is_ok() {
            eprintln!("Editor: loaded level from {}", path);
            return;
        }

        let legacy_path = self.legacy_level_path();
        if self.load_from_path(&legacy_path).is_ok() {
            eprintln!(
                "Editor: loaded level from legacy path {}; use F5 to save under {}",
                legacy_path, path
            );
            return;
        }

        eprintln!(
            "Editor: failed to load {} (and legacy path {})",
            path, legacy_path
        );
    }
}
