use crate::net::{ConnState, NetClient};
use crate::util::scan_first_level;
use game::world::units::px_to_tiles;

use super::App;

impl App {
    pub(super) fn start_lobby_session(&mut self) {
        self.net = Some(NetClient::connect(
            "127.0.0.1:7777".parse().unwrap(),
            "Player".into(),
        ));
        self.net_seq = 0;
        self.server_me = None;
        self.net_bullet_traces.clear();
        self.net_players.clear();
        self.lobby_state = None;
    }

    pub(super) fn sync_lobby_state_from_network(&mut self) {
        if let Some(net) = &self.net {
            for lobby in net.take_lobby_states() {
                self.lobby_state = Some(lobby);
            }
        }
    }

    pub(super) fn is_net_connected(&self) -> bool {
        self.net
            .as_ref()
            .map(|net| matches!(net.state(), ConnState::Connected { .. }))
            .unwrap_or(false)
    }

    pub(super) fn leave_online_session(&mut self) {
        if let Some(net) = self.net.take() {
            net.disconnect();
        }
        self.net_players.clear();
        self.lobby_state = None;
    }

    pub(super) fn enter_play_state(&mut self, sw: f32, sh: f32) {
        if let Some(level) = scan_first_level() {
            self.game
                .load_level(&level, px_to_tiles(sw), px_to_tiles(sh));
        }
        self.camera
            .reset((self.game.player.movement.x, self.game.player.movement.y));
        self.server_me = None;
        self.net_bullet_traces.clear();
        self.net_players.clear();
        self.lobby_state = None;
    }
}
