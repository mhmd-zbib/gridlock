use crate::ui::lobby::LobbyChoice;
use crate::ui::menu::MenuChoice;
use engine::input::InputState;
use game::world::units::px_to_tiles;

use super::{App, AppState};

impl App {
    pub(super) fn tick(
        &mut self,
        sw: f32,
        sh: f32,
        mx: f32,
        my: f32,
        esc: bool,
        enter: bool,
        f1: bool,
        click: bool,
        input: &InputState,
    ) -> Option<AppState> {
        let mut lobby_status = None;
        if matches!(self.app_state, AppState::Lobby(_)) {
            self.sync_lobby_state_from_network();
            let game_started = self
                .lobby_state
                .as_ref()
                .map(|state| state.game_started)
                .unwrap_or(false);
            let can_start = self.is_net_connected() && !game_started;
            lobby_status = Some((game_started, can_start));
        }

        match &mut self.app_state {
            AppState::MainMenu(menu) => {
                if click {
                    match menu.click(sw, sh, mx, my)? {
                        MenuChoice::Play => {
                            self.start_lobby_session();
                            return Some(AppState::Lobby(crate::ui::lobby::LobbyMenu::new()));
                        }
                        MenuChoice::Loadout => {
                            return Some(AppState::Loadout(crate::ui::loadout::LoadoutMenu::new(
                                &self.game.player_loadout(),
                            )));
                        }
                        MenuChoice::Editor => {
                            self.editor.refresh_prop_assets();
                            return Some(AppState::Editing);
                        }
                    }
                }
                None
            }
            AppState::Lobby(menu) => {
                let (game_started, can_start) = lobby_status.unwrap_or((false, false));
                if game_started {
                    self.enter_play_state(sw, sh);
                    return Some(AppState::Playing);
                }

                if click {
                    if let Some(choice) = menu.click(sw, sh, mx, my, can_start) {
                        if let Some(net) = &self.net {
                            match choice {
                                LobbyChoice::Team1 => net.send_lobby_select_team(1),
                                LobbyChoice::Team2 => net.send_lobby_select_team(2),
                                LobbyChoice::StartGame => net.send_lobby_start_game(),
                            }
                        }
                    }
                }

                if esc {
                    self.leave_online_session();
                    return Some(AppState::MainMenu(crate::ui::menu::MainMenu::new()));
                }

                None
            }
            AppState::Loadout(menu) => {
                menu.update(input);
                self.game.set_player_loadout(menu.selected_config());
                if esc || enter {
                    return Some(AppState::MainMenu(crate::ui::menu::MainMenu::new()));
                }
                None
            }
            AppState::Playing => self.tick_playing(sw, sh, mx, my, esc, f1, input),
            AppState::Editing => {
                self.editor.update(input);
                if esc {
                    return Some(AppState::MainMenu(crate::ui::menu::MainMenu::new()));
                }
                if f1 {
                    self.game
                        .load_level(&self.editor.level, px_to_tiles(sw), px_to_tiles(sh));
                    self.camera
                        .reset((self.game.player.movement.x, self.game.player.movement.y));
                    return Some(AppState::Playing);
                }
                None
            }
        }
    }
}
