use game::render::quad::QuadInstance;
use game::world::prop::{self};
use game::world::units::px_to_tiles;

use super::helpers::{
    append_grid_lines, bounds_from_points, effective_snap_mode, preview_edge_walls,
    push_bounds_fill, snap_point,
};
use super::{Editor, Tool};

impl Editor {
    pub fn instances(
        &self,
        screen_w: f32,
        screen_h: f32,
        mouse_x: f32,
        mouse_y: f32,
    ) -> Vec<QuadInstance> {
        let (mouse_x, mouse_y) = self.screen_to_world(mouse_x, mouse_y);
        let snap_mode = effective_snap_mode(self.tool, self.snap_mode);
        let (mx, my) = snap_point(mouse_x, mouse_y, snap_mode);

        let mut out = Vec::new();
        self.push_grid(&mut out, screen_w, screen_h);
        self.push_level_instances(&mut out);
        self.push_tool_preview(&mut out, mx, my, snap_mode);
        out
    }

    fn push_grid(&self, out: &mut Vec<QuadInstance>, screen_w: f32, screen_h: f32) {
        append_grid_lines(
            out,
            screen_w,
            screen_h,
            self.show_subgrid,
            self.view_origin,
            self.zoom,
        );
    }

    fn push_level_instances(&self, out: &mut Vec<QuadInstance>) {
        if let Some(bounds) = self.level.map_bounds {
            push_bounds_fill(
                out,
                bounds,
                [0.10, 0.16, 0.26, 0.16],
                [0.16, 0.42, 0.95, 0.72],
                self.view_origin,
                self.zoom,
            );
        }

        for wall in &self.level.walls {
            out.push(self.world_quad(
                (wall.x + wall.w * 0.5, wall.y + wall.h * 0.5),
                (wall.w * 0.5, wall.h * 0.5),
                if wall.breakable {
                    [0.2, 0.8, 0.95, 1.0]
                } else {
                    [0.45, 0.4, 0.35, 1.0]
                },
            ));
        }

        for prop_instance in &self.level.props {
            let (half_w, half_h, is_collider) = match self.find_prop_asset(&prop_instance.id) {
                Some(asset) => (asset.width * 0.5, asset.height * 0.5, asset.is_collider),
                None => (px_to_tiles(6.0), px_to_tiles(6.0), false),
            };
            out.push(self.world_quad(
                (prop_instance.x, prop_instance.y),
                (half_w, half_h),
                prop::asset_color(&prop_instance.id, is_collider, 1.0),
            ));
        }

        if let Some(spawn) = self.level.player_spawn {
            out.push(self.world_quad(
                (spawn.x, spawn.y),
                (px_to_tiles(10.0), px_to_tiles(10.0)),
                [0.2, 1.0, 0.2, 1.0],
            ));
        }

        for enemy in &self.level.enemies {
            out.push(self.world_quad(
                (enemy.x, enemy.y),
                (px_to_tiles(8.0), px_to_tiles(8.0)),
                [1.0, 0.2, 0.2, 1.0],
            ));
        }

        for dummy in &self.level.target_enemies {
            out.push(self.world_quad(
                (dummy.x, dummy.y),
                (px_to_tiles(8.0), px_to_tiles(8.0)),
                [1.0, 0.85, 0.2, 1.0],
            ));
        }
    }

    fn push_tool_preview(
        &self,
        out: &mut Vec<QuadInstance>,
        mx: f32,
        my: f32,
        snap_mode: super::SnapMode,
    ) {
        match self.tool {
            Tool::Wall => self.push_wall_preview(out, mx, my, snap_mode),
            Tool::Breakable => self.push_breakable_preview(out, mx, my, snap_mode),
            Tool::PlayerSpawn => out.push(self.world_quad(
                (mx, my),
                (px_to_tiles(10.0), px_to_tiles(10.0)),
                [0.2, 1.0, 0.2, 0.35],
            )),
            Tool::Enemy => out.push(self.world_quad(
                (mx, my),
                (px_to_tiles(8.0), px_to_tiles(8.0)),
                [1.0, 0.2, 0.2, 0.35],
            )),
            Tool::TargetDummy => out.push(self.world_quad(
                (mx, my),
                (px_to_tiles(8.0), px_to_tiles(8.0)),
                [1.0, 0.85, 0.2, 0.35],
            )),
            Tool::Prop => self.push_prop_preview(out, mx, my),
            Tool::BaseMap => self.push_bounds_preview(out, mx, my),
        }
    }

    fn push_wall_preview(
        &self,
        out: &mut Vec<QuadInstance>,
        mx: f32,
        my: f32,
        snap_mode: super::SnapMode,
    ) {
        if let Some(start) = self.wall_start {
            for wall in preview_edge_walls(start, (mx, my), snap_mode, false) {
                out.push(self.world_quad(
                    (wall.x + wall.w * 0.5, wall.y + wall.h * 0.5),
                    (wall.w * 0.5, wall.h * 0.5),
                    [0.45, 0.4, 0.35, 0.45],
                ));
            }
            return;
        }

        out.push(self.world_quad(
            (mx, my),
            (px_to_tiles(3.0), px_to_tiles(3.0)),
            [0.45, 0.4, 0.35, 0.5],
        ));
    }

    fn push_breakable_preview(
        &self,
        out: &mut Vec<QuadInstance>,
        mx: f32,
        my: f32,
        snap_mode: super::SnapMode,
    ) {
        if let Some(start) = self.breakable_start {
            for wall in preview_edge_walls(start, (mx, my), snap_mode, true) {
                out.push(self.world_quad(
                    (wall.x + wall.w * 0.5, wall.y + wall.h * 0.5),
                    (wall.w * 0.5, wall.h * 0.5),
                    [0.2, 0.8, 0.95, 0.45],
                ));
            }
            return;
        }

        out.push(self.world_quad(
            (mx, my),
            (px_to_tiles(3.0), px_to_tiles(3.0)),
            [0.2, 0.8, 0.95, 0.5],
        ));
    }

    fn push_prop_preview(&self, out: &mut Vec<QuadInstance>, mx: f32, my: f32) {
        if let Some(asset) = self.selected_prop_definition() {
            out.push(self.world_quad(
                (mx, my),
                (asset.width * 0.5, asset.height * 0.5),
                prop::asset_color(&asset.id, asset.is_collider, 0.45),
            ));
            return;
        }

        out.push(self.world_quad(
            (mx, my),
            (px_to_tiles(5.0), px_to_tiles(5.0)),
            [0.95, 0.2, 0.2, 0.5],
        ));
    }

    fn push_bounds_preview(&self, out: &mut Vec<QuadInstance>, mx: f32, my: f32) {
        if let Some(start) = self.base_map_start {
            if let Some(bounds) = bounds_from_points(start, (mx, my)) {
                push_bounds_fill(
                    out,
                    bounds,
                    [0.12, 0.22, 0.36, 0.28],
                    [0.35, 0.65, 1.0, 0.86],
                    self.view_origin,
                    self.zoom,
                );
            }
            return;
        }

        out.push(self.world_quad(
            (mx, my),
            (px_to_tiles(4.0), px_to_tiles(4.0)),
            [0.35, 0.65, 1.0, 0.55],
        ));
    }
}
