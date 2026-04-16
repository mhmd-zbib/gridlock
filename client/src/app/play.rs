use crate::systems::input::build_client_packet;
use crate::systems::network::apply_server_snapshots;
use crate::util::{enemies_in_combat, infer_world_bounds};
use engine::input::InputState;
use engine::timing::FIXED_STEP;
use game::world::camera::{CameraBehaviorState, CameraStepInput};
use game::world::units::{px_to_tiles, tiles_to_px};
use net::decode_rotation;

use super::{App, AppState};

impl App {
    pub(super) fn tick_playing(
        &mut self,
        sw: f32,
        sh: f32,
        mx: f32,
        my: f32,
        esc: bool,
        f1: bool,
        input: &InputState,
    ) -> Option<AppState> {
        let viewport_px = (sw, sh);
        let mouse_world = self.camera.screen_to_world((mx, my), viewport_px);
        let net_connected = self.is_net_connected();
        let local_input = self.build_local_play_input(input, mouse_world, net_connected);

        self.game.update(FIXED_STEP, &local_input);
        self.consume_server_snapshots();
        self.apply_server_authoritative_player_state(net_connected);
        self.update_bullet_traces();
        self.update_play_camera(viewport_px, sw, sh, mouse_world);
        self.send_play_input(input, mouse_world);

        if esc {
            self.leave_online_session();
            return Some(AppState::MainMenu(crate::ui::menu::MainMenu::new()));
        }
        if f1 {
            self.editor.refresh_prop_assets();
            return Some(AppState::Editing);
        }
        if input.f8 && !self.prev_f8 {
            self.debug_mode = !self.debug_mode;
        }
        None
    }

    fn build_local_play_input(
        &self,
        input: &InputState,
        mouse_world: (f32, f32),
        net_connected: bool,
    ) -> InputState {
        let mut local_input = input.clone();
        local_input.mouse_x = tiles_to_px(mouse_world.0) as f64;
        local_input.mouse_y = tiles_to_px(mouse_world.1) as f64;

        if net_connected {
            local_input.shoot = false;
            local_input.right = false;
            local_input.left = false;
            local_input.up = false;
            local_input.down = false;
        }

        local_input
    }

    fn consume_server_snapshots(&mut self) {
        if let Some(net) = &self.net {
            apply_server_snapshots(
                net,
                &mut self.server_me,
                &mut self.net_players,
                &mut self.net_bullet_traces,
            );
        }
    }

    fn apply_server_authoritative_player_state(&mut self, net_connected: bool) {
        if !net_connected {
            return;
        }

        if let Some(me) = self.server_me.as_ref() {
            self.game.player.movement.x = me.x;
            self.game.player.movement.y = me.y;
            let rotation = decode_rotation(me.rotation);
            self.game.player.sight.direction = rotation;
            self.game.player.aim_cone.direction = rotation;
        }
    }

    fn update_bullet_traces(&mut self) {
        for trace in &mut self.net_bullet_traces {
            trace.ttl -= FIXED_STEP;
        }
        self.net_bullet_traces.retain(|trace| trace.ttl > 0.0);
    }

    fn update_play_camera(
        &mut self,
        viewport_px: (f32, f32),
        sw: f32,
        sh: f32,
        mouse_world: (f32, f32),
    ) {
        let desired_state = if enemies_in_combat(&self.game.enemies) {
            CameraBehaviorState::Combat
        } else {
            CameraBehaviorState::Exploration
        };

        self.camera.update(CameraStepInput {
            dt: FIXED_STEP,
            viewport_px,
            player_pos: (self.game.player.movement.x, self.game.player.movement.y),
            mouse_world,
            player_vision_range: self.game.player.sight.range,
            bounds: infer_world_bounds(&self.game, px_to_tiles(sw), px_to_tiles(sh)),
            rooms: &self.game.rooms,
            desired_state,
        });
    }

    fn send_play_input(&mut self, input: &InputState, mouse_world: (f32, f32)) {
        if let Some(net) = &self.net {
            if matches!(net.state(), crate::net::ConnState::Connected { .. }) {
                let packet = build_client_packet(self.net_seq, input, mouse_world, &self.game);
                net.send_input(packet);
                self.net_seq = self.net_seq.wrapping_add(1);
            }
        }
    }
}
