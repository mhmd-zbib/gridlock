mod assets;
mod helpers;
mod level_edit;
mod render;
mod tool;
mod update;
mod viewport;

pub use tool::{SnapMode, Tool};

use std::collections::HashMap;

use game::world::floor::{self as floor_world, FloorAssetDef};
use game::world::level::LevelData;
use game::world::prop::{self, PropAssetDef};

use helpers::effective_snap_mode;
use tool::{EdgeCell, EdgeKey};

pub struct Editor {
    pub tool: Tool,
    pub level: LevelData,
    pub snap_mode: SnapMode,
    pub show_subgrid: bool,
    pub zoom: f32,
    prop_assets: Vec<PropAssetDef>,
    selected_prop_asset: usize,
    floor_assets: Vec<FloorAssetDef>,
    selected_floor_asset: usize,
    edges: HashMap<EdgeKey, EdgeCell>,
    view_origin: (f32, f32),

    wall_start: Option<(f32, f32)>,
    breakable_start: Option<(f32, f32)>,
    base_map_start: Option<(f32, f32)>,

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
    prev_key_g: bool,
    prev_key_h: bool,
}

impl Editor {
    pub fn new() -> Self {
        let prop_assets = prop::load_assets();
        let floor_assets = floor_world::load_assets();
        Self {
            tool: Tool::default(),
            level: LevelData::default(),
            snap_mode: SnapMode::Edge,
            show_subgrid: true,
            zoom: 1.0,
            prop_assets,
            selected_prop_asset: 0,
            floor_assets,
            selected_floor_asset: 0,
            edges: HashMap::new(),
            view_origin: (0.0, 0.0),
            wall_start: None,
            breakable_start: None,
            base_map_start: None,
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
            prev_key_g: false,
            prev_key_h: false,
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

    pub fn active_snap_label(&self) -> &'static str {
        effective_snap_mode(self.tool, self.snap_mode).label()
    }

    pub fn save(&self, path: &str) {
        let _ = self.level.save(path);
    }

    pub fn load(&mut self, path: &str) {
        if let Ok(data) = LevelData::load(path) {
            self.level = data;
            self.rebuild_edges_from_walls();
            self.rebuild_walls_from_edges();
        }
    }
}
