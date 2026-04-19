use crate::camera::{CameraBehaviorState, CameraStepInput};
use crate::net::ConnState;
use crate::render::entities::{NET_BULLET_TTL, NetBulletTrace};
use crate::systems::input::build_client_packet;
use crate::systems::network::apply_server_snapshots;
use crate::systems::prediction::{
    predict_local_player, push_pending_input, reconcile_local_player,
};
use crate::util::{enemies_in_combat, infer_world_bounds};
use game::game::BULLET_MAX_RANGE;
use game::input::InputState;
use game::timing::FIXED_STEP;
use game::world::ray::cast_ray;
use game::world::units::{px_to_tiles, tiles_to_px};
use net::ClientPacket;

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

        if net_connected {
            // Predict locally for smooth movement, then reconcile to snapshots.
            if let Some(packet) = self.send_play_input(input, mouse_world) {
                let authoritative_health = self.server_me.map(|me| me.health);
                push_pending_input(&mut self.pending_local_inputs, packet);
                predict_local_player(
                    &mut self.game,
                    &packet,
                    &mut self.predicted_peek_origin,
                    authoritative_health,
                );
                self.tick_predicted_shot_feedback(&packet);
            }
            if let Some(latest_ack_timestamp) = self.consume_server_snapshots() {
                self.reconcile_predicted_player_state(latest_ack_timestamp);
            }
            self.smooth_remote_players();
            self.update_bullet_traces();
            self.tick_kill_notification();
        } else {
            // Single-player: game runs the full simulation locally.
            self.pending_local_inputs.clear();
            self.predicted_peek_origin = None;
            self.predicted_shot_cooldown = 0.0;
            self.last_server_tick = None;
            self.remote_player_targets.clear();
            self.net_players.clear();
            let mut local_input = input.clone();
            local_input.mouse_x = tiles_to_px(mouse_world.0) as f64;
            local_input.mouse_y = tiles_to_px(mouse_world.1) as f64;
            self.game.update(FIXED_STEP, &local_input);
        }

        self.update_play_camera(viewport_px, sw, sh);

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

    fn consume_server_snapshots(&mut self) -> Option<u32> {
        let my_player_id = self.net.as_ref().and_then(|n| {
            if let ConnState::Connected { player_id } = n.state() {
                Some(player_id)
            } else {
                None
            }
        });
        if let Some(net) = &self.net {
            let result = apply_server_snapshots(
                net,
                &mut self.server_me,
                &mut self.remote_player_targets,
                &mut self.teammate_sight_cones,
                &mut self.net_bullet_traces,
                &mut self.match_state,
                &mut self.last_server_tick,
                self.my_team,
                my_player_id,
            );
            if let Some(killer) = result.killer {
                self.kill_notification = Some((killer, 3.0));
            }
            return result.latest_ack_timestamp;
        }
        None
    }

    fn tick_kill_notification(&mut self) {
        if let Some((_, remaining)) = &mut self.kill_notification {
            *remaining -= FIXED_STEP;
            if *remaining <= 0.0 {
                self.kill_notification = None;
            }
        }
    }

    fn reconcile_predicted_player_state(&mut self, latest_ack_timestamp: u32) {
        if let Some(server_me) = self.server_me {
            reconcile_local_player(
                &mut self.game,
                server_me,
                &mut self.pending_local_inputs,
                &mut self.predicted_peek_origin,
                latest_ack_timestamp,
            );
        }
    }

    fn update_bullet_traces(&mut self) {
        for trace in &mut self.net_bullet_traces {
            trace.ttl -= FIXED_STEP;
        }
        self.net_bullet_traces.retain(|trace| trace.ttl > 0.0);
    }

    fn tick_predicted_shot_feedback(&mut self, input_packet: &ClientPacket) {
        self.predicted_shot_cooldown = (self.predicted_shot_cooldown - FIXED_STEP).max(0.0);
        if !input_packet.flags.is_shooting() || self.predicted_shot_cooldown > 0.0 {
            return;
        }
        if self.server_me.is_some_and(|me| me.health == 0) {
            return;
        }

        let stats = self.game.player.active_weapon_stats();
        self.predicted_shot_cooldown = 1.0 / stats.fire_rate_rps.max(0.01);

        let from = (self.game.player.movement.x, self.game.player.movement.y);
        let dir = (
            self.game.player.aim_cone.direction.cos(),
            self.game.player.aim_cone.direction.sin(),
        );
        let dist = cast_ray(from, dir, BULLET_MAX_RANGE, &self.game.walls);
        self.net_bullet_traces.push(NetBulletTrace {
            from_x: from.0,
            from_y: from.1,
            to_x: from.0 + dir.0 * dist,
            to_y: from.1 + dir.1 * dist,
            ttl: NET_BULLET_TTL * 0.8,
            friendly: true,
        });
    }

    fn smooth_remote_players(&mut self) {
        const REMOTE_SMOOTH_BLEND: f32 = 0.35;
        const REMOTE_HARD_SNAP_DIST: f32 = px_to_tiles(180.0);
        let hard_snap_sq = REMOTE_HARD_SNAP_DIST * REMOTE_HARD_SNAP_DIST;

        if self.remote_player_targets.is_empty() {
            self.net_players.clear();
            return;
        }

        let mut smoothed = Vec::with_capacity(self.remote_player_targets.len());
        for target in &self.remote_player_targets {
            let next = if let Some(current) = self.net_players.iter().find(|p| p.id == target.id) {
                let dx = target.x - current.x;
                let dy = target.y - current.y;
                if dx * dx + dy * dy > hard_snap_sq {
                    *target
                } else {
                    let mut blended = *target;
                    blended.x = current.x + dx * REMOTE_SMOOTH_BLEND;
                    blended.y = current.y + dy * REMOTE_SMOOTH_BLEND;
                    blended
                }
            } else {
                *target
            };
            smoothed.push(next);
        }
        self.net_players = smoothed;
    }

    fn update_play_camera(&mut self, viewport_px: (f32, f32), sw: f32, sh: f32) {
        let desired_state = if enemies_in_combat(&self.game.enemies) {
            CameraBehaviorState::Combat
        } else {
            CameraBehaviorState::Exploration
        };

        self.camera.update(CameraStepInput {
            viewport_px,
            player_pos: (self.game.player.movement.x, self.game.player.movement.y),
            bounds: infer_world_bounds(&self.game, px_to_tiles(sw), px_to_tiles(sh)),
            rooms: &self.game.rooms,
            desired_state,
        });
    }

    fn send_play_input(
        &mut self,
        input: &InputState,
        mouse_world: (f32, f32),
    ) -> Option<ClientPacket> {
        if let Some(net) = &self.net {
            if matches!(net.state(), crate::net::ConnState::Connected { .. }) {
                let packet = build_client_packet(self.net_seq, input, mouse_world, &self.game);
                net.send_input(packet);
                self.net_seq = self.net_seq.wrapping_add(1);
                return Some(packet);
            }
        }
        None
    }
}
