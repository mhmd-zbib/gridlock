mod play;
mod render;
mod session;
mod tick;

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Fullscreen, Window, WindowId};

use crate::camera::TacticalCamera;
use crate::net::NetClient;
use crate::render::entities::NetBulletTrace;
use crate::ui::editor::Editor;
use crate::ui::loadout::LoadoutMenu;
use crate::ui::lobby::LobbyMenu;
use crate::ui::menu::MainMenu;
use game::game::Game;
use engine::asset::AssetHandle;
use engine::input::InputHandler;
use engine::render::state::State;
use engine::timing::GameLoop;
use net::{ClientPacket, LobbyState, MatchState, PlayerState, SelfState, TeammateView};

// ---------------------------------------------------------------------------
// App state machine
// ---------------------------------------------------------------------------

enum AppState {
    MainMenu(MainMenu),
    NameEntry,
    Loadout(LoadoutMenu),
    Lobby(LobbyMenu),
    Playing,
    Editing,
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

pub struct App {
    wgpu_state: Option<State>,
    input: InputHandler,
    game: Game,
    camera: TacticalCamera,
    editor: Editor,
    app_state: AppState,
    net: Option<NetClient>,

    game_loop: GameLoop,

    prev_esc: bool,
    prev_enter: bool,
    prev_f1: bool,
    prev_f8: bool,
    prev_click: bool,
    prev_backspace: bool,

    debug_mode: bool,
    net_seq: u16,
    pending_local_inputs: VecDeque<ClientPacket>,
    predicted_peek_origin: Option<(f32, f32)>,
    predicted_shot_cooldown: f32,
    last_server_tick: Option<u32>,
    net_bullet_traces: Vec<NetBulletTrace>,
    server_me: Option<SelfState>,
    remote_player_targets: Vec<PlayerState>,
    net_players: Vec<PlayerState>,
    teammate_sight_cones: Vec<TeammateView>,
    lobby_state: Option<LobbyState>,
    match_state: Option<MatchState>,
    /// "YOU HAVE BEEN KILLED BY [name]" — Some((name, remaining_secs)).
    kill_notification: Option<(String, f32)>,

    /// Persisted player display name (editable on the name-entry screen).
    player_name: String,
    /// This client's team (`0` = none, `1` = team 1, `2` = team 2).
    my_team: u8,
    /// prop id → GPU texture handle, populated once after the GPU is ready.
    prop_textures: HashMap<String, AssetHandle>,
    /// floor id → GPU texture handle, populated once after the GPU is ready.
    floor_textures: HashMap<String, AssetHandle>,
    _hud_deps: ::ui::hud::HudDependencies,
}

impl Default for App {
    fn default() -> Self {
        let game = Game::new();
        let player_pos = (game.player.movement.x, game.player.movement.y);
        Self {
            wgpu_state: None,
            input: InputHandler::new(),
            game,
            camera: TacticalCamera::new(player_pos),
            editor: Editor::new(),
            app_state: AppState::MainMenu(MainMenu::new()),
            net: None,
            game_loop: GameLoop::new(),
            prev_esc: false,
            prev_enter: false,
            prev_f1: false,
            prev_f8: false,
            prev_click: false,
            prev_backspace: false,
            debug_mode: false,
            net_seq: 0,
            pending_local_inputs: VecDeque::new(),
            predicted_peek_origin: None,
            predicted_shot_cooldown: 0.0,
            last_server_tick: None,
            net_bullet_traces: Vec::new(),
            server_me: None,
            remote_player_targets: Vec::new(),
            net_players: Vec::new(),
            teammate_sight_cones: Vec::new(),
            lobby_state: None,
            match_state: None,
            kill_notification: None,
            player_name: default_player_name(),
            my_team: 0,
            prop_textures: HashMap::new(),
            floor_textures: HashMap::new(),
            _hud_deps: ::ui::hud::HudDependencies::tactical_defaults(),
        }
    }
}

impl App {
    /// Upload every prop texture that has a path in its asset def.
    /// Silently skips props whose PNG is not present on disk yet — those
    /// props fall back to the solid-color quad renderer until the file arrives.
    fn load_prop_textures(&mut self) {
        let Some(state) = self.wgpu_state.as_mut() else {
            return;
        };
        let prop_defs = game::world::prop::load_assets();
        for def in prop_defs {
            if let Some(ref path) = def.texture {
                if let Some(handle) = state.load_texture(path) {
                    self.prop_textures.insert(def.id, handle);
                }
            }
        }
    }

    fn load_floor_textures(&mut self) {
        let Some(state) = self.wgpu_state.as_mut() else {
            return;
        };
        let floor_defs = game::world::floor::load_assets();
        for def in floor_defs {
            if let Some(ref path) = def.texture {
                if let Some(handle) = state.load_texture(path) {
                    self.floor_textures.insert(def.id, handle);
                }
            }
        }
    }
}

fn default_player_name() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u32)
        .unwrap_or(1234);
    format!("Player{}", n % 9999 + 1)
}

// ---------------------------------------------------------------------------
// winit integration
// ---------------------------------------------------------------------------

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("Shooting")
                        .with_fullscreen(Some(Fullscreen::Borderless(None))),
                )
                .unwrap(),
        );
        self.wgpu_state = Some(pollster::block_on(State::new(window)));
        self.load_prop_textures();
        self.load_floor_textures();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if self.input.handle(&event) {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(new_size) => {
                if let Some(s) = self.wgpu_state.as_mut() {
                    s.resize(new_size);
                }
            }
            WindowEvent::RedrawRequested => {
                let Some(state) = self.wgpu_state.as_mut() else {
                    return;
                };
                let sw = state.size.width as f32;
                let sh = state.size.height as f32;
                let window = Arc::clone(&state.window);
                let _ = state; // release borrow so we can call &mut self methods

                self.run_fixed_updates(sw, sh);

                // --- Uncapped render loop ---
                let input = self.input.state.clone();
                let mx = input.mouse_x as f32;
                let my = input.mouse_y as f32;
                self.render_frame(sw, sh, mx, my);
                window.request_redraw();
            }
            _ => {}
        }
    }
}

impl App {
    fn run_fixed_updates(&mut self, sw: f32, sh: f32) {
        let steps = self.game_loop.consume_fixed_steps();
        for _ in 0..steps {
            let input = self.input.state.clone();
            let mx = input.mouse_x as f32;
            let my = input.mouse_y as f32;

            let esc = input.escape && !self.prev_esc;
            let enter = input.enter && !self.prev_enter;
            let f1 = input.f1 && !self.prev_f1;
            let click = input.mouse_left && !self.prev_click;
            let backspace = input.backspace && !self.prev_backspace;

            if let Some(next_state) =
                self.tick(sw, sh, mx, my, esc, enter, f1, click, backspace, &input)
            {
                self.app_state = next_state;
            }

            self.prev_esc = input.escape;
            self.prev_enter = input.enter;
            self.prev_f1 = input.f1;
            self.prev_f8 = input.f8;
            self.prev_click = input.mouse_left;
            self.prev_backspace = input.backspace;
            self.input.end_frame();
        }
    }

    fn render_frame(&mut self, sw: f32, sh: f32, mx: f32, my: f32) {
        const OUTSIDE_CONE_DIM: f32 = 0.60;
        let viewport_px = (sw, sh);
        let (scene_quads, wall_quads, masked_quads) = self.build_quads(viewport_px, mx, my);
        let scene_shaded_quads = self.build_shaded_quads(viewport_px, mx, my);
        let fov_mask = self.build_mask(viewport_px);
        let (geo, masked_geo) = self.build_geo(viewport_px);
        let (floor_sprites, prop_sprites) = self.build_sprites(viewport_px);
        let lighting = self.build_lighting(viewport_px);
        let texts = self.build_texts(sw, sh, mx, my);

        if let Some(state) = self.wgpu_state.as_mut() {
            state.render_frame(&engine::render::frame::Frame {
                fov_mask,
                outside_dim: OUTSIDE_CONE_DIM,
                lighting,
                scene_quads,
                scene_gradient_quads: Vec::new(),
                scene_shaded_quads,
                masked_quads,
                floor_sprites,
                prop_sprites,
                wall_quads,
                geo,
                masked_geo,
                texts,
            });
        }
    }
}
