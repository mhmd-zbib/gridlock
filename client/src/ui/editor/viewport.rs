use game::render::quad::QuadInstance;
use game::world::units::{px_to_tiles, tiles_to_px};

use super::Editor;

impl Editor {
    pub(super) fn screen_to_world(&self, sx: f32, sy: f32) -> (f32, f32) {
        (
            self.view_origin.0 + px_to_tiles(sx) / self.zoom,
            self.view_origin.1 + px_to_tiles(sy) / self.zoom,
        )
    }

    fn world_to_screen(&self, wx: f32, wy: f32) -> (f32, f32) {
        (
            tiles_to_px(wx - self.view_origin.0) * self.zoom,
            tiles_to_px(wy - self.view_origin.1) * self.zoom,
        )
    }

    fn world_len_to_screen(&self, world_len: f32) -> f32 {
        tiles_to_px(world_len).abs() * self.zoom
    }

    pub(super) fn world_quad(
        &self,
        center: (f32, f32),
        half_size: (f32, f32),
        color: [f32; 4],
    ) -> QuadInstance {
        let (cx, cy) = self.world_to_screen(center.0, center.1);
        QuadInstance {
            center: [cx, cy],
            half_size: [
                self.world_len_to_screen(half_size.0),
                self.world_len_to_screen(half_size.1),
            ],
            color,
        }
    }
}
