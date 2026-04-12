use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

use crate::editor::Editor;
use crate::game::Game;
use crate::game_loop::GameLoop;
use crate::geometry::{push_circle_fan, push_cone_fan, GeoVertex};
use crate::input::InputHandler;
use crate::level_select::LevelSelect;
use crate::menu::{MainMenu, MenuChoice};
use crate::renderer::QuadInstance;
use crate::state::State;
use crate::text::TextSection;

macro_rules! ts {
    ($x:expr, $y:expr, $text:expr, $size:expr, $color:expr) => {
        TextSection { x: $x, y: $y, text: $text.to_string(), size: $size, color: $color }
    };
}

// ---------------------------------------------------------------------------
// App state machine
// ---------------------------------------------------------------------------

enum AppState {
    MainMenu(MainMenu),
    LevelSelect(LevelSelect),
    Playing,
    Editing,
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

pub struct App {
    wgpu_state: Option<State>,
    input:      InputHandler,
    game:       Game,
    game_loop:  GameLoop,
    editor:     Editor,
    app_state:  AppState,

    prev_esc:   bool,
    prev_enter: bool,
    prev_f1:    bool,
    prev_click: bool,
}

impl Default for App {
    fn default() -> Self {
        Self {
            wgpu_state: None,
            input:      InputHandler::new(),
            game:       Game::new(),
            game_loop:  GameLoop::new(),
            editor:     Editor::new(),
            app_state:  AppState::MainMenu(MainMenu::new()),
            prev_esc:   false,
            prev_enter: false,
            prev_f1:    false,
            prev_click: false,
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
                .create_window(Window::default_attributes().with_title("Shooting"))
                .unwrap(),
        );
        self.wgpu_state = Some(pollster::block_on(State::new(window)));
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id:        WindowId,
        event:      WindowEvent,
    ) {
        if self.input.handle(&event) { return; }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(new_size) => {
                if let Some(s) = self.wgpu_state.as_mut() { s.resize(new_size); }
            }
            WindowEvent::RedrawRequested => {
                let Some(state) = self.wgpu_state.as_mut() else { return; };
                let sw = state.size.width  as f32;
                let sh = state.size.height as f32;
                let window = Arc::clone(&state.window);
                let _ = state; // release borrow so we can call &mut self methods

                let input = self.input.state.clone();
                let mx    = input.mouse_x as f32;
                let my    = input.mouse_y as f32;

                let esc   = input.escape     && !self.prev_esc;
                let enter = input.enter      && !self.prev_enter;
                let f1    = input.f1         && !self.prev_f1;
                let click = input.mouse_left && !self.prev_click;

                let next = self.tick(sw, sh, mx, my, esc, enter, f1, click, &input);

                let quads = self.build_quads(sw, sh, mx, my);
                let geo   = self.build_geo();
                let texts = self.build_texts(sw, sh, mx, my);

                self.wgpu_state.as_mut().unwrap().render(&quads, &geo, &texts);

                if let Some(ns) = next { self.app_state = ns; }

                self.prev_esc   = input.escape;
                self.prev_enter = input.enter;
                self.prev_f1    = input.f1;
                self.prev_click = input.mouse_left;

                window.request_redraw();
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Logic
// ---------------------------------------------------------------------------

impl App {
    fn tick(
        &mut self,
        sw: f32, sh: f32, mx: f32, my: f32,
        esc: bool, enter: bool, f1: bool, click: bool,
        input: &crate::input::InputState,
    ) -> Option<AppState> {
        match &mut self.app_state {
            AppState::MainMenu(menu) => {
                if click {
                    match menu.click(sw, sh, mx, my)? {
                        MenuChoice::Play => {
                            let mut sel = LevelSelect::new();
                            sel.refresh();
                            return Some(AppState::LevelSelect(sel));
                        }
                        MenuChoice::Editor => {
                            return Some(AppState::Editing);
                        }
                    }
                }
                None
            }

            AppState::LevelSelect(sel) => {
                sel.navigate(input.up, input.down);
                if esc   { return Some(AppState::MainMenu(MainMenu::new())); }
                if enter {
                    if let Some(path) = sel.selected_path() {
                        match crate::level::LevelData::load(path) {
                            Ok(level) => {
                                self.game.load_level(&level);
                                return Some(AppState::Playing);
                            }
                            Err(e) => println!("[app] load error: {e}"),
                        }
                    }
                }
                None
            }

            AppState::Playing => {
                let game = &mut self.game;
                let gl   = &mut self.game_loop;
                gl.tick(|dt| game.update(dt, input), || {});
                if esc { return Some(AppState::MainMenu(MainMenu::new())); }
                if f1  { return Some(AppState::Editing); }
                None
            }

            AppState::Editing => {
                self.editor.update(input);
                if esc { return Some(AppState::MainMenu(MainMenu::new())); }
                if f1  {
                    self.game.load_level(&self.editor.level);
                    return Some(AppState::Playing);
                }
                None
            }
        }
    }

    // ---------------------------------------------------------------------------
    // Sight-cone geometry (triangles)
    // ---------------------------------------------------------------------------

    fn build_geo(&self) -> Vec<GeoVertex> {
        if let AppState::Playing = &self.app_state {
            play_geo(&self.game)
        } else {
            Vec::new()
        }
    }

    // ---------------------------------------------------------------------------
    // Quad geometry
    // ---------------------------------------------------------------------------

    fn build_quads(&self, sw: f32, sh: f32, mx: f32, my: f32) -> Vec<QuadInstance> {
        match &self.app_state {
            AppState::MainMenu(menu)   => menu.instances(sw, sh, mx, my),
            AppState::LevelSelect(sel) => sel.instances(sw, sh),
            AppState::Playing          => play_quads(&self.game),
            AppState::Editing          => self.editor.instances(mx, my),
        }
    }

    // ---------------------------------------------------------------------------
    // Text labels
    // ---------------------------------------------------------------------------

    fn build_texts(&self, sw: f32, sh: f32, _mx: f32, _my: f32) -> Vec<TextSection> {
        match &self.app_state {
            AppState::MainMenu(_)      => main_menu_texts(sw, sh),
            AppState::LevelSelect(sel) => level_select_texts(sw, sh, sel),
            AppState::Playing          => play_texts(sw, sh),
            AppState::Editing          => editor_texts(sw, sh, &self.editor),
        }
    }
}

// ---------------------------------------------------------------------------
// Per-screen text builders
// ---------------------------------------------------------------------------

fn main_menu_texts(sw: f32, sh: f32) -> Vec<TextSection> {
    let cx = sw * 0.5;
    vec![
        ts!(cx - 160.0, sh * 0.12,               "SHOOTING GAME", 48.0, [1.0, 1.0, 1.0, 1.0]),
        ts!(cx -  68.0, sh * 0.35 + sh*0.12*0.28, "PLAY GAME",     28.0, [0.0, 0.0, 0.0, 1.0]),
        ts!(cx -  92.0, sh * 0.55 + sh*0.12*0.28, "LEVEL EDITOR",  28.0, [0.0, 0.0, 0.0, 1.0]),
    ]
}

fn level_select_texts(sw: f32, sh: f32, sel: &LevelSelect) -> Vec<TextSection> {
    let mut out = vec![
        ts!(sw*0.5 - 110.0, sh*0.07, "SELECT LEVEL", 36.0, [1.0, 1.0, 1.0, 1.0]),
        ts!(sw*0.5 - 200.0, sh*0.90,
            "Up/Down: navigate   Enter: load   Esc: back",
            14.0, [0.55, 0.55, 0.55, 1.0]),
    ];

    let bh      = (sh * 0.60 / sel.levels.len().max(1) as f32).min(52.0);
    let gap     = 6.0;
    let start_y = sh * 0.20;
    let lx      = sw * 0.5 - sw * 0.55 * 0.5 + 8.0;

    for (i, path) in sel.levels.iter().enumerate() {
        let name  = path.file_stem().and_then(|s| s.to_str()).unwrap_or("?").to_string();
        let row_y = start_y + i as f32 * (bh + gap);
        let ty    = row_y + (bh - 18.0) * 0.5;
        let color = if i == sel.selected { [0.05, 0.05, 0.05, 1.0] } else { [0.75, 0.75, 0.75, 1.0] };
        out.push(TextSection { x: lx, y: ty, text: name, size: 20.0, color });
    }
    out
}

fn play_texts(sw: f32, _sh: f32) -> Vec<TextSection> {
    vec![
        ts!(8.0, 6.0, "WASD: move   click: shoot   Esc: menu   F1: editor", 13.0, [0.5, 0.5, 0.5, 1.0]),
        ts!(sw - 170.0, 6.0, "SHOOTING GAME", 13.0, [0.35, 0.35, 0.35, 1.0]),
    ]
}

fn editor_texts(sw: f32, sh: f32, editor: &Editor) -> Vec<TextSection> {
    let tool = match editor.tool {
        crate::editor::Tool::PlayerSpawn => "Player Spawn",
        crate::editor::Tool::Enemy       => "Enemy",
        crate::editor::Tool::Wall        => "Wall (drag)",
    };
    let grid  = if editor.grid_snap { "Grid: ON" } else { "Grid: OFF" };
    let stats = format!("Enemies: {}  Walls: {}", editor.level.enemies.len(), editor.level.walls.len());
    vec![
        ts!(8.0,       6.0,      "LEVEL EDITOR",   18.0, [1.0, 0.7, 0.2, 1.0]),
        ts!(8.0,      28.0,      tool,              15.0, [1.0, 1.0, 1.0, 1.0]),
        ts!(8.0,      46.0,      grid,              13.0, [0.6, 0.6, 0.6, 1.0]),
        ts!(6.0,  sh-38.0, "1: Spawn   2: Enemy   3: Wall   G: Grid", 13.0, [0.55, 0.55, 0.55, 1.0]),
        ts!(6.0,  sh-20.0, "Left: place   Right: delete   F5: save   L: load   F1: play   Esc: menu", 13.0, [0.55, 0.55, 0.55, 1.0]),
        ts!(sw-180.0,  6.0,      stats,             13.0, [0.5, 0.5, 0.5, 1.0]),
    ]
}

// ---------------------------------------------------------------------------
// Play-mode quad builder
// ---------------------------------------------------------------------------

fn play_quads(game: &Game) -> Vec<QuadInstance> {
    let mut out = Vec::new();
    for w in &game.walls {
        out.push(QuadInstance {
            center:    [w.x + w.w * 0.5, w.y + w.h * 0.5],
            half_size: [w.w * 0.5, w.h * 0.5],
            color:     [0.45, 0.4, 0.35, 1.0],
        });
    }
    out.push(QuadInstance {
        center:    [game.player.movement.x, game.player.movement.y],
        half_size: [10.0, 10.0],
        color:     [1.0, 1.0, 1.0, 1.0],
    });
    for e in &game.enemies {
        if !e.visible_to_player { continue; }
        out.push(QuadInstance {
            center:    [e.movement.x, e.movement.y],
            half_size: [8.0, 8.0],
            color:     [1.0, 0.2, 0.2, 1.0],
        });
    }
    for b in &game.bullets {
        out.push(QuadInstance {
            center:    [b.x, b.y],
            half_size: [3.0, 3.0],
            color:     [1.0, 1.0, 0.0, 1.0],
        });
    }
    out
}

// ---------------------------------------------------------------------------
// Play-mode sight-cone geometry builder
// ---------------------------------------------------------------------------

fn play_geo(game: &Game) -> Vec<GeoVertex> {
    let mut out   = Vec::new();
    let walls     = &game.walls;
    let player_pos = (game.player.movement.x, game.player.movement.y);

    // --- Player ---
    // Nearby circle (soft blue, very translucent).
    push_circle_fan(&mut out, player_pos, game.player.sight.circle_radius,
        [0.3, 0.7, 1.0, 0.07], 48);
    // Vision cone.
    let arc = game.player.sight.cone_arc_pts(player_pos, walls, 60);
    push_cone_fan(&mut out, player_pos, &arc, [0.3, 0.7, 1.0, 0.16]);

    // --- Enemies (only those the player can see) ---
    for e in &game.enemies {
        if !e.visible_to_player { continue; }
        let ep = (e.movement.x, e.movement.y);

        // Nearby circle.
        push_circle_fan(&mut out, ep, e.sight.circle_radius,
            [1.0, 0.3, 0.3, 0.05], 36);

        // Cone color: red if chasing, orange if alerted, yellow if patrolling.
        let cone_color = if e.sees_player {
            [1.0, 0.08, 0.08, 0.35]
        } else if e.is_alerted() {
            [1.0, 0.50, 0.05, 0.28]
        } else {
            [1.0, 0.85, 0.20, 0.14]
        };
        let arc = e.sight.cone_arc_pts(ep, walls, 48);
        push_cone_fan(&mut out, ep, &arc, cone_color);
    }

    out
}
