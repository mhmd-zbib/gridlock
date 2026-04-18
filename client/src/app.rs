mod play;
mod render;
mod session;
mod tick;

use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Fullscreen, Window, WindowId};

use crate::camera::TacticalCamera;
use crate::net::NetClient;
use crate::render::entities::NetBulletTrace;
use crate::render::geometry::OUTSIDE_CONE_DIM;
use crate::ui::editor::Editor;
use crate::ui::loadout::LoadoutMenu;
use crate::ui::lobby::LobbyMenu;
use crate::ui::menu::MainMenu;
use engine::input::InputHandler;
use engine::render::state::State;
use engine::timing::GameLoop;
use game::game::Game;
use net::{LobbyState, PlayerState, SelfState};

// ---------------------------------------------------------------------------
// App state machine
// ---------------------------------------------------------------------------

enum AppState {
    MainMenu(MainMenu),
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

    debug_mode: bool,
    net_seq: u16,
    net_bullet_traces: Vec<NetBulletTrace>,
    server_me: Option<SelfState>,
    net_players: Vec<PlayerState>,
    lobby_state: Option<LobbyState>,
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
            debug_mode: false,
            net_seq: 0,
            net_bullet_traces: Vec::new(),
            server_me: None,
            net_players: Vec::new(),
            lobby_state: None,
        }
    }
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

            if let Some(next_state) = self.tick(sw, sh, mx, my, esc, enter, f1, click, &input) {
                self.app_state = next_state;
            }

            self.prev_esc = input.escape;
            self.prev_enter = input.enter;
            self.prev_f1 = input.f1;
            self.prev_f8 = input.f8;
            self.prev_click = input.mouse_left;
            self.input.end_frame();
        }
    }

    fn render_frame(&mut self, sw: f32, sh: f32, mx: f32, my: f32) {
        let viewport_px = (sw, sh);
        let (scene_quads, masked_quads) = self.build_quads(viewport_px, mx, my);
        let mask = self.build_mask(viewport_px);
        let (geo, masked_geo) = self.build_geo(viewport_px);
        let texts = self.build_texts(sw, sh, mx, my);

        if let Some(state) = self.wgpu_state.as_mut() {
            state.render(
                &mask,
                &scene_quads,
                &masked_quads,
                &geo,
                &masked_geo,
                &texts,
                OUTSIDE_CONE_DIM,
            );
        }
    }
}
