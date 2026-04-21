use std::collections::HashMap;

use game::render::quad::QuadInstance;
use game::render::sprite::{AssetHandle, SpriteCommand};
use game::world::floor;
use game::world::prop;
use game::world::units::px_to_tiles;

use super::helpers::{
    append_grid_lines, bounds_from_points, effective_snap_mode, push_bounds_fill, snap_point,
};
use super::{Editor, Tool};

impl Editor {
    /// Returns `(scene_quads, top_quads)`.
    /// `scene_quads` — floors, props, spawns and non-wall tool previews.
    /// `top_quads`   — walls, wall previews and grid (rendered above sprites).
    pub fn layered_instances(
        &self,
        screen_w: f32,
        screen_h: f32,
        mouse_x: f32,
        mouse_y: f32,
    ) -> (Vec<QuadInstance>, Vec<QuadInstance>) {
        let (raw_mx, raw_my) = self.screen_to_world(mouse_x, mouse_y);
        let snap_mode = effective_snap_mode(self.tool, self.snap_mode);
        let (mx, my) = snap_point(raw_mx, raw_my, snap_mode);

        let mut scene = Vec::new();
        let mut top = Vec::new();

        self.push_level_scene(&mut scene);
        self.push_level_walls(&mut top);
        self.push_tool_preview(&mut scene, &mut top, raw_mx, raw_my, mx, my);
        // Grid goes last in top so it overlays everything (including walls).
        self.push_grid(&mut top, screen_w, screen_h);

        (scene, top)
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

    fn push_level_scene(&self, out: &mut Vec<QuadInstance>) {
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

        let bounds_alpha = if self.show_textures { 0.22 } else { 1.0 };

        for floor_instance in &self.level.floors {
            let (half_w, half_h) = match self.find_floor_asset(&floor_instance.id) {
                Some(asset) => (asset.width * 0.5, asset.height * 0.5),
                None => (0.5, 0.5),
            };
            let mut color = floor::asset_color(&floor_instance.id, 1.0);
            color[3] = bounds_alpha;
            out.push(self.world_quad(
                (floor_instance.x, floor_instance.y),
                (half_w, half_h),
                color,
            ));
        }

        for prop_instance in &self.level.props {
            let (half_w, half_h, is_collider) = match self.find_prop_asset(&prop_instance.id) {
                Some(asset) => (asset.width * 0.5, asset.height * 0.5, asset.is_collider),
                None => (px_to_tiles(6.0), px_to_tiles(6.0), false),
            };
            let mut color = prop::asset_color(&prop_instance.id, is_collider, 1.0);
            color[3] = bounds_alpha;
            out.push(self.world_quad(
                (prop_instance.x, prop_instance.y),
                (half_w, half_h),
                color,
            ));
        }

        if let Some(spawn) = self.level.player_spawn {
            out.push(self.world_quad(
                (spawn.x, spawn.y),
                (px_to_tiles(10.0), px_to_tiles(10.0)),
                [0.2, 1.0, 0.2, 1.0],
            ));
        }

        if let Some(spawn) = self.level.team1_spawn {
            out.push(self.world_quad(
                (spawn.x, spawn.y),
                (px_to_tiles(10.0), px_to_tiles(10.0)),
                [0.2, 0.5, 1.0, 1.0],
            ));
        }

        if let Some(spawn) = self.level.team2_spawn {
            out.push(self.world_quad(
                (spawn.x, spawn.y),
                (px_to_tiles(10.0), px_to_tiles(10.0)),
                [1.0, 0.5, 0.1, 1.0],
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

    fn push_level_walls(&self, out: &mut Vec<QuadInstance>) {
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
    }

    fn push_tool_preview(
        &self,
        scene: &mut Vec<QuadInstance>,
        top: &mut Vec<QuadInstance>,
        raw_mx: f32,
        raw_my: f32,
        mx: f32,
        my: f32,
    ) {
        match self.tool {
            Tool::Wall => self.push_wall_block_preview(top, mx, my, false),
            Tool::Breakable => self.push_wall_block_preview(top, mx, my, true),
            Tool::PlayerSpawn => scene.push(self.world_quad(
                (mx, my),
                (px_to_tiles(10.0), px_to_tiles(10.0)),
                [0.2, 1.0, 0.2, 0.35],
            )),
            Tool::Enemy => scene.push(self.world_quad(
                (mx, my),
                (px_to_tiles(8.0), px_to_tiles(8.0)),
                [1.0, 0.2, 0.2, 0.35],
            )),
            Tool::TargetDummy => scene.push(self.world_quad(
                (mx, my),
                (px_to_tiles(8.0), px_to_tiles(8.0)),
                [1.0, 0.85, 0.2, 0.35],
            )),
            Tool::Team1Spawn => scene.push(self.world_quad(
                (mx, my),
                (px_to_tiles(10.0), px_to_tiles(10.0)),
                [0.2, 0.5, 1.0, 0.35],
            )),
            Tool::Team2Spawn => scene.push(self.world_quad(
                (mx, my),
                (px_to_tiles(10.0), px_to_tiles(10.0)),
                [1.0, 0.5, 0.1, 0.35],
            )),
            Tool::Prop => self.push_prop_preview(top, mx, my),
            Tool::Floor => self.push_floor_preview(top, mx, my),
            Tool::BaseMap => self.push_bounds_preview(scene, mx, my),
            Tool::Eraser => {
                let half = self.eraser_size_steps as f32 * super::tool::EDGE_STEP * 0.5;
                top.push(self.world_quad((raw_mx, raw_my), (half, half), [1.0, 0.2, 0.2, 0.25]));
            }
        }
    }

    fn push_wall_block_preview(
        &self,
        out: &mut Vec<QuadInstance>,
        mx: f32,
        my: f32,
        breakable: bool,
    ) {
        let size = self.wall_block_size();
        let half = size * 0.5;
        let color = if breakable {
            [0.2_f32, 0.8, 0.95, 0.5]
        } else {
            [0.45_f32, 0.4, 0.35, 0.5]
        };
        let dim_color = [color[0], color[1], color[2], color[3] * 0.9];

        let drag_start = if breakable { self.breakable_drag_start } else { self.wall_drag_start };

        if let Some((sx, sy)) = drag_start {
            let ex = mx;
            let ey = my;
            let dx = (ex - sx).abs();
            let dy = (ey - sy).abs();

            if dx >= dy {
                let (from_x, to_x) = if sx <= ex { (sx, ex) } else { (ex, sx) };
                let mut x = from_x;
                while x <= to_x + 0.001 {
                    out.push(self.world_quad((x + half, sy + half), (half, half), dim_color));
                    x += size;
                }
            } else {
                let (from_y, to_y) = if sy <= ey { (sy, ey) } else { (ey, sy) };
                let mut y = from_y;
                while y <= to_y + 0.001 {
                    out.push(self.world_quad((sx + half, y + half), (half, half), dim_color));
                    y += size;
                }
            }
            return;
        }

        out.push(self.world_quad((mx + half, my + half), (half, half), color));
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

    fn push_floor_preview(&self, out: &mut Vec<QuadInstance>, mx: f32, my: f32) {
        if let Some(asset) = self.selected_floor_definition() {
            out.push(self.world_quad(
                (mx, my),
                (asset.width * 0.5, asset.height * 0.5),
                floor::asset_color(&asset.id, 0.45),
            ));
            return;
        }

        out.push(self.world_quad((mx, my), (0.5, 0.5), [0.4, 0.3, 0.2, 0.5]));
    }

    /// Emit textured sprite commands for floors and props visible in the editor.
    pub fn build_editor_sprites(
        &self,
        floor_textures: &HashMap<String, AssetHandle>,
        prop_textures: &HashMap<String, AssetHandle>,
    ) -> (Vec<SpriteCommand>, Vec<SpriteCommand>) {
        if !self.show_textures {
            return (vec![], vec![]);
        }
        let mut floor_cmds = Vec::new();
        let mut prop_cmds = Vec::new();

        for f in &self.level.floors {
            let Some(&handle) = floor_textures.get(&f.id) else {
                continue;
            };
            let asset_half = match self.find_floor_asset(&f.id) {
                Some(a) => (a.width * 0.5, a.height * 0.5),
                None => (0.5, 0.5),
            };
            let (cx, cy) = self.world_to_screen(f.x, f.y);
            let hw = self.world_len_to_screen(asset_half.0);
            let hh = self.world_len_to_screen(asset_half.1);
            let mut cmd = SpriteCommand::simple(handle, [cx, cy], [hw, hh], 0.0);
            cmd.uv_rect = floor_uv_rect(asset_half);
            floor_cmds.push(cmd);
        }

        for p in &self.level.props {
            let Some(&handle) = prop_textures.get(&p.id) else {
                continue;
            };
            let asset_half = match self.find_prop_asset(&p.id) {
                Some(a) => (a.width * 0.5, a.height * 0.5),
                None => (px_to_tiles(6.0), px_to_tiles(6.0)),
            };
            let (cx, cy) = self.world_to_screen(p.x, p.y);
            let hw = self.world_len_to_screen(asset_half.0);
            let hh = self.world_len_to_screen(asset_half.1);
            prop_cmds.push(SpriteCommand::simple(handle, [cx, cy], [hw, hh], 1.0));
        }

        (floor_cmds, prop_cmds)
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

fn floor_uv_rect(half_size: (f32, f32)) -> [f32; 4] {
    let wu = (half_size.0 * 2.0).max(0.0);
    let hu = (half_size.1 * 2.0).max(0.0);
    let us = wu.min(1.0);
    let vs = hu.min(1.0);
    [0.5 - us * 0.5, 0.5 - vs * 0.5, 0.5 + us * 0.5, 0.5 + vs * 0.5]
}
