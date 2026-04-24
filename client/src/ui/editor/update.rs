use engine::input::InputState;
use game::world::units::px_to_tiles;

use super::helpers::{bounds_from_points, effective_snap_mode, snap_point};
use super::tool::{
    EDITOR_KEY_ZOOM_STEP, EDITOR_MAX_ZOOM, EDITOR_MIN_ZOOM, EDITOR_PAN_STEP_PX,
    EDITOR_WHEEL_ZOOM_STEP, MAX_ERASER_STEPS, cycle_wall_step,
};
use super::{Editor, Tool};

impl Editor {
    pub fn update(&mut self, input: &InputState) {
        let mouse_px = (input.mouse_x as f32, input.mouse_y as f32);

        self.update_zoom(input, mouse_px);
        self.update_pan(input);

        let (raw_mx, raw_my) = self.screen_to_world(mouse_px.0, mouse_px.1);
        let snap_mode = effective_snap_mode(self.tool, self.snap_mode);
        let (mx, my) = snap_point(raw_mx, raw_my, snap_mode);

        let just_pressed = input.mouse_left && !self.prev_left;
        let just_released = !input.mouse_left && self.prev_left;
        let just_right_pressed = input.mouse_right && !self.prev_right;

        self.handle_tool_selection(input);
        self.handle_snap_toggles(input);
        self.handle_prop_cycle(input);
        self.handle_placement(just_pressed, just_released, input.shift, raw_mx, raw_my, mx, my);
        self.handle_floor_drag(input.mouse_left, mx, my);
        self.handle_erase(input.mouse_left, raw_mx, raw_my);
        self.handle_right_drag(input.mouse_right, just_right_pressed, mx, my, raw_mx, raw_my);
        self.handle_save_load(input);
        self.update_prev_input_state(input);
    }

    fn update_zoom(&mut self, input: &InputState, mouse_px: (f32, f32)) {
        let mut zoom_factor = 1.0_f32;
        if input.key_equals {
            zoom_factor *= EDITOR_KEY_ZOOM_STEP;
        }
        if input.key_minus {
            zoom_factor /= EDITOR_KEY_ZOOM_STEP;
        }
        if input.mouse_wheel_y != 0.0 {
            zoom_factor *= (1.0 + input.mouse_wheel_y * EDITOR_WHEEL_ZOOM_STEP).max(0.2);
        }

        if (zoom_factor - 1.0).abs() <= f32::EPSILON {
            return;
        }

        let world_before = self.screen_to_world(mouse_px.0, mouse_px.1);
        self.zoom = (self.zoom * zoom_factor).clamp(EDITOR_MIN_ZOOM, EDITOR_MAX_ZOOM);
        self.view_origin = (
            world_before.0 - px_to_tiles(mouse_px.0) / self.zoom,
            world_before.1 - px_to_tiles(mouse_px.1) / self.zoom,
        );
    }

    fn update_pan(&mut self, input: &InputState) {
        let pan_step_world = px_to_tiles(EDITOR_PAN_STEP_PX) / self.zoom.max(0.001);
        if input.left {
            self.view_origin.0 -= pan_step_world;
        }
        if input.right {
            self.view_origin.0 += pan_step_world;
        }
        if input.up {
            self.view_origin.1 -= pan_step_world;
        }
        if input.down {
            self.view_origin.1 += pan_step_world;
        }
    }

    fn handle_tool_selection(&mut self, input: &InputState) {
        if input.key_1 && !self.prev_key_1 {
            self.select_tool(Tool::PlayerSpawn);
        }
        if input.key_2 && !self.prev_key_2 {
            self.select_tool(Tool::Enemy);
        }
        if input.key_3 && !self.prev_key_3 {
            self.select_tool(Tool::Wall);
        }
        if input.key_4 && !self.prev_key_4 {
            self.select_tool(Tool::TargetDummy);
        }
        if input.key_5 && !self.prev_key_5 {
            self.select_tool(Tool::Breakable);
        }
        if input.key_6 && !self.prev_key_6 {
            self.select_tool(Tool::Prop);
            self.announce_selected_prop_asset();
        }
        if input.key_7 && !self.prev_key_7 {
            self.select_tool(Tool::BaseMap);
        }
        if input.key_8 && !self.prev_key_8 {
            self.select_tool(Tool::Team1Spawn);
        }
        if input.key_9 && !self.prev_key_9 {
            self.select_tool(Tool::Team2Spawn);
        }
        if input.key_0 && !self.prev_key_0 {
            self.select_tool(Tool::Floor);
        }
        if input.key_f && !self.prev_key_f {
            self.select_tool(Tool::Eraser);
        }
    }

    fn handle_snap_toggles(&mut self, input: &InputState) {
        if input.key_g && !self.prev_key_g {
            self.snap_mode = self.snap_mode.toggle();
        }
        if input.key_h && !self.prev_key_h {
            self.show_subgrid = !self.show_subgrid;
        }
        if input.key_t && !self.prev_key_t {
            self.show_textures = !self.show_textures;
        }
    }

    fn handle_prop_cycle(&mut self, input: &InputState) {
        match self.tool {
            Tool::Prop => {
                if input.key_q && !self.prev_key_q {
                    self.cycle_prop_asset(-1);
                }
                if input.key_e && !self.prev_key_e {
                    self.cycle_prop_asset(1);
                }
            }
            Tool::Floor => {
                if input.key_q && !self.prev_key_q {
                    self.cycle_floor_asset(-1);
                }
                if input.key_e && !self.prev_key_e {
                    self.cycle_floor_asset(1);
                }
            }
            Tool::Wall | Tool::Breakable => {
                if input.key_q && !self.prev_key_q {
                    self.wall_size_steps = cycle_wall_step(self.wall_size_steps);
                }
                if input.key_e && !self.prev_key_e {
                    self.wall_size_steps = cycle_wall_step(self.wall_size_steps);
                }
            }
            Tool::Eraser => {
                if input.key_q && !self.prev_key_q && self.eraser_size_steps > 1 {
                    self.eraser_size_steps -= 1;
                }
                if input.key_e && !self.prev_key_e && self.eraser_size_steps < MAX_ERASER_STEPS {
                    self.eraser_size_steps += 1;
                }
            }
            _ => {}
        }
    }

    fn handle_placement(
        &mut self,
        just_pressed: bool,
        just_released: bool,
        shift: bool,
        raw_mx: f32,
        raw_my: f32,
        mx: f32,
        my: f32,
    ) {
        match self.tool {
            Tool::Wall => {
                if just_released {
                    if let Some(start) = self.wall_drag_start.take() {
                        self.place_wall_line(start, (mx, my), false);
                    }
                } else if just_pressed {
                    if shift {
                        self.wall_drag_start = Some((mx, my));
                    } else {
                        self.place_wall_block(mx, my, false);
                    }
                }
            }
            Tool::Breakable => {
                if just_released {
                    if let Some(start) = self.breakable_drag_start.take() {
                        self.place_wall_line(start, (mx, my), true);
                    }
                } else if just_pressed {
                    if shift {
                        self.breakable_drag_start = Some((mx, my));
                    } else {
                        self.place_wall_block(mx, my, true);
                    }
                }
            }
            Tool::BaseMap => {
                if just_pressed {
                    if let Some(start) = self.base_map_start.take() {
                        if let Some(bounds) = bounds_from_points(start, (mx, my)) {
                            self.level.map_bounds = Some(bounds);
                        }
                    } else {
                        self.base_map_start = Some((mx, my));
                    }
                }
            }
            Tool::Floor => {} // handled by handle_floor_drag
            _ => {
                if just_pressed {
                    self.place(mx, my);
                }
            }
        }
    }

    fn handle_floor_drag(&mut self, mouse_held: bool, mx: f32, my: f32) {
        if self.tool != Tool::Floor {
            self.floor_drag_last = None;
            return;
        }
        if !mouse_held {
            self.floor_drag_last = None;
            return;
        }
        if self.floor_drag_last == Some((mx, my)) {
            return;
        }
        self.floor_drag_last = Some((mx, my));
        self.place(mx, my);
    }

    fn handle_erase(&mut self, mouse_held: bool, raw_mx: f32, raw_my: f32) {
        if self.tool != Tool::Eraser || !mouse_held {
            return;
        }
        let half = self.eraser_size_steps as f32 * super::tool::EDGE_STEP * 0.5;
        self.erase_in_rect(raw_mx, raw_my, half);
    }

    fn handle_right_drag(
        &mut self,
        mouse_right: bool,
        just_right_pressed: bool,
        mx: f32,
        my: f32,
        raw_mx: f32,
        raw_my: f32,
    ) {
        // Cancel in-progress left-drag on right press.
        if just_right_pressed {
            match self.tool {
                Tool::Wall => { self.wall_drag_start.take(); }
                Tool::Breakable => { self.breakable_drag_start.take(); }
                Tool::BaseMap => {
                    if self.base_map_start.take().is_none() {
                        self.level.map_bounds.take();
                    }
                    return;
                }
                // One-shot deletes — only fire on first press.
                Tool::Eraser => { self.delete_at(raw_mx, raw_my); return; }
                Tool::PlayerSpawn => { self.delete_player_spawn_at(mx, my); return; }
                Tool::Team1Spawn => { self.delete_team_spawn_at(mx, my, 1); return; }
                Tool::Team2Spawn => { self.delete_team_spawn_at(mx, my, 2); return; }
                _ => {}
            }
        }

        // Continuous drag deletion — fires every subgrid cell the mouse enters.
        if !mouse_right {
            self.right_drag_last = None;
            return;
        }
        if self.right_drag_last == Some((mx, my)) {
            return;
        }
        self.right_drag_last = Some((mx, my));

        match self.tool {
            Tool::Wall => self.delete_wall_at_point(mx, my, false),
            Tool::Breakable => self.delete_wall_at_point(mx, my, true),
            Tool::Floor => self.delete_nearest_floor_at(mx, my),
            Tool::Prop => self.delete_nearest_prop_at(mx, my),
            Tool::Enemy => self.delete_nearest_enemy_at(mx, my),
            Tool::TargetDummy => self.delete_nearest_target_dummy_at(mx, my),
            _ => {}
        }
    }

    fn handle_save_load(&mut self, input: &InputState) {
        if input.f5 && !self.prev_f5 {
            self.save_active_level();
        }
        if input.key_l && !self.prev_key_l {
            self.load_active_level();
        }
    }

    fn update_prev_input_state(&mut self, input: &InputState) {
        self.prev_left = input.mouse_left;
        self.prev_right = input.mouse_right;
        self.prev_f5 = input.f5;
        self.prev_key_l = input.key_l;
        self.prev_key_1 = input.key_1;
        self.prev_key_2 = input.key_2;
        self.prev_key_3 = input.key_3;
        self.prev_key_4 = input.key_4;
        self.prev_key_5 = input.key_5;
        self.prev_key_6 = input.key_6;
        self.prev_key_7 = input.key_7;
        self.prev_key_8 = input.key_8;
        self.prev_key_9 = input.key_9;
        self.prev_key_0 = input.key_0;
        self.prev_key_q = input.key_q;
        self.prev_key_e = input.key_e;
        self.prev_key_f = input.key_f;
        self.prev_key_g = input.key_g;
        self.prev_key_h = input.key_h;
        self.prev_key_t = input.key_t;
    }
}
