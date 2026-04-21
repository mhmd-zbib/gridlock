use game::world::floor::FloorAssetDef;
use game::world::prop::PropAssetDef;

use super::{Editor, Tool};

impl Editor {
    pub(super) fn selected_prop_definition(&self) -> Option<&PropAssetDef> {
        self.prop_assets.get(self.selected_prop_asset)
    }

    pub(super) fn find_prop_asset(&self, id: &str) -> Option<&PropAssetDef> {
        self.prop_assets.iter().find(|asset| asset.id == id)
    }

    pub(super) fn cycle_prop_asset(&mut self, dir: i32) {
        if self.prop_assets.is_empty() {
            return;
        }
        let len = self.prop_assets.len() as i32;
        self.selected_prop_asset =
            ((self.selected_prop_asset as i32 + dir).rem_euclid(len)) as usize;
        self.announce_selected_prop_asset();
    }

    pub(super) fn announce_selected_prop_asset(&self) {}

    pub(super) fn selected_floor_definition(&self) -> Option<&FloorAssetDef> {
        self.floor_assets.get(self.selected_floor_asset)
    }

    pub(super) fn find_floor_asset(&self, id: &str) -> Option<&FloorAssetDef> {
        self.floor_assets.iter().find(|asset| asset.id == id)
    }

    pub(super) fn cycle_floor_asset(&mut self, dir: i32) {
        if self.floor_assets.is_empty() {
            return;
        }
        let len = self.floor_assets.len() as i32;
        self.selected_floor_asset =
            ((self.selected_floor_asset as i32 + dir).rem_euclid(len)) as usize;
    }

    pub(super) fn select_tool(&mut self, tool: Tool) {
        self.tool = tool;
        self.wall_drag_start = None;
        self.breakable_drag_start = None;
        self.base_map_start = None;
    }
}
