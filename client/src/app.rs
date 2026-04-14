use std::sync::Arc;
use std::time::Instant;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Fullscreen, Window, WindowId};

use crate::net::{ConnState, NetClient};
use net::{ClientPacket, InputFlags, SelfState, encode_rotation};
use crate::ui::editor::{Editor, Tool};
use crate::ui::loadout::LoadoutMenu;
use crate::ui::menu::{MainMenu, MenuChoice};
use engine::input::InputHandler;
use engine::render::geometry::{GeoVertex, push_circle_fan, push_cone_fan};
use engine::render::quad::QuadInstance;
use engine::render::state::State;
use engine::render::text::TextSection;
use engine::timing::FIXED_STEP;
use game::entity::enemy::EnemyKind;
use game::entity::weapon::attachment::AttachmentCategory;
use game::game::Game;
use game::world::camera::{CameraBehaviorState, CameraBounds, CameraStepInput, TacticalCamera};
use game::world::level::LevelData;
use game::world::prop;
use game::world::rooms::LevelRooms;
use game::world::units::{px_to_tiles, tiles_to_px};

macro_rules! ts {
    ($x:expr, $y:expr, $text:expr, $size:expr, $color:expr) => {
        TextSection {
            x: $x,
            y: $y,
            text: $text.to_string(),
            size: $size,
            color: $color,
        }
    };
}

// ---------------------------------------------------------------------------
// Net bullet trace (server-authoritative)
// ---------------------------------------------------------------------------

/// A bullet trace received from the server, kept alive for a short display TTL.
struct NetBulletTrace {
    from_x: f32,
    from_y: f32,
    to_x: f32,
    to_y: f32,
    ttl: f32,
}

const NET_BULLET_TTL: f32 = 0.30;
/// Max screen-space length (px) of a rendered bullet trace.  Prevents traces
/// from shooting far off screen when the server has no walls.
const NET_BULLET_MAX_SCREEN_PX: f32 = 500.0;

// ---------------------------------------------------------------------------
// App state machine
// ---------------------------------------------------------------------------

enum AppState {
    MainMenu(MainMenu),
    Loadout(LoadoutMenu),
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

    input_timer: Instant,
    input_accumulator: f32,

    prev_esc: bool,
    prev_enter: bool,
    prev_f1: bool,
    prev_f8: bool,
    prev_click: bool,

    debug_mode: bool,
    net_seq: u16,
    /// Bullet traces received from the server, with remaining display time.
    net_bullet_traces: Vec<NetBulletTrace>,
    /// Latest authoritative self-state received from the server (ammo, health, …).
    server_me: Option<SelfState>,
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
            input_timer: Instant::now(),
            input_accumulator: 0.0,
            prev_esc: false,
            prev_enter: false,
            prev_f1: false,
            prev_f8: false,
            prev_click: false,
            debug_mode: false,
            net_seq: 0,
            net_bullet_traces: Vec::new(),
            server_me: None,
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

                // --- Input loop (60 Hz) ---
                let now = Instant::now();
                let frame_dt = now.duration_since(self.input_timer).as_secs_f32().min(0.25);
                self.input_timer = now;
                self.input_accumulator += frame_dt;

                while self.input_accumulator >= FIXED_STEP {
                    let input = self.input.state.clone();
                    let mx = input.mouse_x as f32;
                    let my = input.mouse_y as f32;

                    let esc   = input.escape     && !self.prev_esc;
                    let enter = input.enter      && !self.prev_enter;
                    let f1    = input.f1         && !self.prev_f1;
                    let click = input.mouse_left && !self.prev_click;

                    if let Some(ns) = self.tick(sw, sh, mx, my, esc, enter, f1, click, &input) {
                        self.app_state = ns;
                    }

                    self.prev_esc   = input.escape;
                    self.prev_enter = input.enter;
                    self.prev_f1    = input.f1;
                    self.prev_f8    = input.f8;
                    self.prev_click = input.mouse_left;
                    self.input.end_frame();

                    self.input_accumulator -= FIXED_STEP;
                }

                // --- Render loop (uncapped) ---
                let input = self.input.state.clone();
                let mx = input.mouse_x as f32;
                let my = input.mouse_y as f32;

                let quads = self.build_quads(sw, sh, mx, my);
                let geo   = self.build_geo(sw, sh);
                let texts = self.build_texts(sw, sh, mx, my);

                self.wgpu_state.as_mut().unwrap().render(&quads, &geo, &texts);

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
        sw: f32,
        sh: f32,
        mx: f32,
        my: f32,
        esc: bool,
        enter: bool,
        f1: bool,
        click: bool,
        input: &engine::input::InputState,
    ) -> Option<AppState> {
        match &mut self.app_state {
            AppState::MainMenu(menu) => {
                if click {
                    match menu.click(sw, sh, mx, my)? {
                        MenuChoice::Play => {
                            // Start network connection
                            self.net = Some(NetClient::connect(
                                "127.0.0.1:7777".parse().unwrap(),
                                "Player".into(),
                            ));
                            self.net_seq = 0;

                            // Load first available level, fall back to default game state
                            if let Some(level) = scan_first_level() {
                                self.game.load_level(
                                    &level,
                                    px_to_tiles(sw),
                                    px_to_tiles(sh),
                                );
                            }
                            self.camera.reset((
                                self.game.player.movement.x,
                                self.game.player.movement.y,
                            ));
                            return Some(AppState::Playing);
                        }
                        MenuChoice::Loadout => {
                            return Some(AppState::Loadout(LoadoutMenu::new(
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

            AppState::Loadout(menu) => {
                menu.update(input);
                self.game.set_player_loadout(menu.selected_config());
                if esc || enter {
                    return Some(AppState::MainMenu(MainMenu::new()));
                }
                None
            }

            AppState::Playing => {
                let viewport_px = (sw, sh);
                let mouse_world = self.camera.screen_to_world(
                    (input.mouse_x as f32, input.mouse_y as f32),
                    viewport_px,
                );
                let mut world_input = input.clone();
                world_input.mouse_x = tiles_to_px(mouse_world.0) as f64;
                world_input.mouse_y = tiles_to_px(mouse_world.1) as f64;

                let net_connected = self
                    .net
                    .as_ref()
                    .map(|n| matches!(n.state(), ConnState::Connected { .. }))
                    .unwrap_or(false);

                // When connected, bullet visuals come from the server.  Strip
                // `shoot` from the local sim so it does not spawn projectiles or
                // generate local impact marks — ammo/reload state still update
                // because we keep `reload` in world_input.
                let mut local_input = world_input.clone();
                if net_connected {
                    local_input.shoot = false;
                }

                self.game.update(FIXED_STEP, &local_input);

                // Pull server snapshots and ingest bullet events.
                if let Some(net) = &self.net {
                    if let Some(snap) = net.take_snapshot() {
                        self.server_me = Some(snap.me);
                        for b in snap.bullets {
                            self.net_bullet_traces.push(NetBulletTrace {
                                from_x: b.from_x,
                                from_y: b.from_y,
                                to_x: b.to_x,
                                to_y: b.to_y,
                                ttl: NET_BULLET_TTL,
                            });
                        }
                    }
                }

                // Age and prune bullet traces.
                for t in &mut self.net_bullet_traces {
                    t.ttl -= FIXED_STEP;
                }
                self.net_bullet_traces.retain(|t| t.ttl > 0.0);

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
                // Send 60 Hz input to the server when connected
                if let Some(net) = &self.net {
                    if matches!(net.state(), ConnState::Connected { .. }) {
                        let mut flags = InputFlags::default();
                        flags.set_shooting(input.shoot);
                        flags.set_reloading(input.reload);
                        flags.set_walking(input.walk);
                        flags.set_peeking(input.peek);

                        let angle = (mouse_world.1 - self.game.player.movement.y)
                            .atan2(mouse_world.0 - self.game.player.movement.x);

                        net.send_input(ClientPacket {
                            sequence:   self.net_seq,
                            timestamp:  client_time_ms(),
                            movement_x: (input.right as i8) - (input.left as i8),
                            movement_y: (input.down  as i8) - (input.up   as i8),
                            rotation:   encode_rotation(angle),
                            flags,
                        });
                        self.net_seq = self.net_seq.wrapping_add(1);
                    }
                }

                if esc {
                    if let Some(net) = self.net.take() {
                        net.disconnect();
                    }
                    return Some(AppState::MainMenu(MainMenu::new()));
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

            AppState::Editing => {
                self.editor.update(input);
                if esc {
                    return Some(AppState::MainMenu(MainMenu::new()));
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

    // ---------------------------------------------------------------------------
    // Sight-cone geometry (triangles)
    // ---------------------------------------------------------------------------

    fn build_geo(&self, sw: f32, sh: f32) -> Vec<GeoVertex> {
        if let AppState::Playing = &self.app_state {
            play_geo(&self.game, &self.camera, self.debug_mode, sw, sh, &self.net_bullet_traces)
        } else {
            vec![]
        }
    }

    // ---------------------------------------------------------------------------
    // Quad geometry
    // ---------------------------------------------------------------------------

    fn build_quads(&self, sw: f32, sh: f32, mx: f32, my: f32) -> Vec<QuadInstance> {
        match &self.app_state {
            AppState::MainMenu(menu) => menu.instances(sw, sh, mx, my),
            AppState::Loadout(loadout) => loadout.instances(sw, sh),
            AppState::Playing => play_quads(&self.game, &self.camera, self.debug_mode, sw, sh),
            AppState::Editing => self.editor.instances(sw, sh, mx, my),
        }
    }

    // ---------------------------------------------------------------------------
    // Text labels
    // ---------------------------------------------------------------------------

    fn build_texts(&self, sw: f32, sh: f32, _mx: f32, _my: f32) -> Vec<TextSection> {
        match &self.app_state {
            AppState::MainMenu(_) => main_menu_texts(sw, sh),
            AppState::Loadout(loadout) => loadout_texts(sw, sh, loadout),
            AppState::Playing => play_texts(sw, sh, &self.game, &self.camera, self.debug_mode, self.net.as_ref(), self.server_me.as_ref()),
            AppState::Editing => editor_texts(sw, sh, &self.editor),
        }
    }
}

// ---------------------------------------------------------------------------
// Per-screen text builders
// ---------------------------------------------------------------------------

fn main_menu_texts(sw: f32, sh: f32) -> Vec<TextSection> {
    let cx = sw * 0.5;
    let bh = sh * 0.10;
    let gap = sh * 0.05;
    let y_play    = sh * 0.28 + bh * 0.26;
    let y_loadout = y_play    + bh + gap;
    let y_editor  = y_loadout + bh + gap;
    vec![
        ts!(cx - 160.0, sh * 0.12, "SHOOTING GAME", 48.0, [1.0, 1.0, 1.0, 1.0]),
        ts!(cx - 68.0,  y_play,    "PLAY GAME",     28.0, [0.0, 0.0, 0.0, 1.0]),
        ts!(cx - 62.0,  y_loadout, "LOADOUT",       28.0, [0.0, 0.0, 0.0, 1.0]),
        ts!(cx - 92.0,  y_editor,  "LEVEL EDITOR",  28.0, [0.0, 0.0, 0.0, 1.0]),
    ]
}

fn loadout_texts(sw: f32, sh: f32, loadout: &LoadoutMenu) -> Vec<TextSection> {
    let mut out = vec![
        ts!(
            sw * 0.5 - 145.0,
            sh * 0.10,
            "LOADOUT BUILDER",
            42.0,
            [1.0, 1.0, 1.0, 1.0]
        ),
        ts!(
            sw * 0.5 - 220.0,
            sh * 0.88,
            "Up/Down: row   Left/Right: option   Enter/Esc: back",
            15.0,
            [0.55, 0.55, 0.55, 1.0]
        ),
    ];

    let bw = sw * 0.64;
    let bh = (sh * 0.55 / 7.0).min(48.0);
    let gap = 8.0;
    let start_y = sh * 0.24;
    let text_x = sw * 0.5 - bw * 0.5 + 16.0;

    let weapon_line = format!(
        "Weapon: {} ({})",
        loadout.selected_weapon_name(),
        loadout.selected_weapon_class_label()
    );
    out.push(ts!(
        text_x,
        start_y + (bh - 18.0) * 0.5,
        weapon_line,
        20.0,
        if loadout.selected_row() == 0 {
            [0.05, 0.05, 0.05, 1.0]
        } else {
            [0.82, 0.82, 0.82, 1.0]
        }
    ));

    for (idx, category) in AttachmentCategory::all().iter().enumerate() {
        let row = idx + 1;
        let row_y = start_y + row as f32 * (bh + gap);
        let supported = loadout.selected_weapon_supports(*category);
        let line = format!(
            "{}: {}",
            category.label(),
            loadout.selected_attachment_name(*category)
        );
        out.push(ts!(
            text_x,
            row_y + (bh - 18.0) * 0.5,
            line,
            20.0,
            if !supported {
                [0.45, 0.18, 0.18, 1.0]
            } else if loadout.selected_row() == row {
                [0.05, 0.05, 0.05, 1.0]
            } else {
                [0.82, 0.82, 0.82, 1.0]
            }
        ));
    }

    out
}

fn play_texts(
    sw: f32,
    sh: f32,
    game: &Game,
    camera: &TacticalCamera,
    debug: bool,
    net: Option<&NetClient>,
    server_me: Option<&SelfState>,
) -> Vec<TextSection> {
    let _ = sh;
    let attachments_line = AttachmentCategory::all()
        .iter()
        .map(|category| {
            format!(
                "{}={}",
                category.label(),
                game.player.attachment_name_for(*category).unwrap_or("-")
            )
        })
        .collect::<Vec<_>>()
        .join("  ");

    // Use server-authoritative ammo when connected; fall back to local game state.
    let ammo_display = if let Some(me) = server_me {
        me.ammo
    } else {
        game.player.ammo_in_mag() as u8
    };
    let reloading = server_me.map(|me| me.reload_progress > 0).unwrap_or(game.player.is_reloading());

    let mut out = vec![
        ts!(
            8.0,
            6.0,
            "WASD: move   Shift: sprint   X+WASD: peek   hold click: fire   Esc: menu   F1: editor   F8: debug",
            13.0,
            [0.5, 0.5, 0.5, 1.0]
        ),
        ts!(
            8.0,
            22.0,
            format!(
                "{} ({})  ammo: {}/{}{}   R: reload",
                game.player.weapon_name(),
                game.player.weapon_class_label(),
                ammo_display,
                game.player.mag_size(),
                if reloading { " [reloading]" } else { "" }
            ),
            13.0,
            [0.5, 0.5, 0.5, 1.0]
        ),
        ts!(8.0, 38.0, attachments_line, 13.0, [0.45, 0.45, 0.45, 1.0]),
        ts!(sw - 170.0, 6.0, "SHOOTING GAME", 13.0, [0.35, 0.35, 0.35, 1.0]),
        ts!(
            sw - 210.0,
            20.0,
            match net.map(|n| n.state()) {
                None                                   => "net: off".into(),
                Some(ConnState::Connecting)            => "net: connecting…".into(),
                Some(ConnState::Connected { player_id }) => format!("net: player #{player_id}"),
                Some(ConnState::Rejected(r))           => format!("net: rejected ({r})"),
                Some(ConnState::Disconnected)          => "net: disconnected".into(),
            },
            12.0,
            [0.35, 0.35, 0.35, 1.0]
        ),
    ];

    if !debug {
        return out;
    }

    // ── Debug info panel ────────────────────────────────────────────────────
    let px = sw - 310.0;
    let mut py = 28.0;
    let lh = 13.0; // line height

    let spd = game.player.movement.speed * game.player.movement.velocity_frac;
    out.push(ts!(
        px,
        py,
        format!(
            "[DEBUG]  spd:{:.2} tiles/s  enemies:{}",
            spd,
            game.enemies.len()
        ),
        12.0,
        [0.9, 0.9, 0.2, 1.0]
    ));
    py += lh + 2.0;

    // Show if player is in a room
    let player_pos = (game.player.movement.x, game.player.movement.y);
    let room_info = match game.rooms.find_room_at(player_pos.0, player_pos.1) {
        Some(room_idx) => format!("Room: {}", room_idx),
        None => "Room: ---".to_string(),
    };
    out.push(ts!(px, py, room_info, 11.0, [0.6, 0.9, 0.6, 1.0]));
    py += lh + 2.0;

    let camera_state = match camera.state() {
        CameraBehaviorState::Combat => "combat",
        CameraBehaviorState::PeekTension => "peek",
        CameraBehaviorState::Exploration => "explore",
    };
    let cam_center = camera.center();
    let cam_offset = camera.offset();
    out.push(ts!(
        px,
        py,
        format!(
            "Cam: {}  center({:.2},{:.2})  off({:.2},{:.2})  room:{}  gap:{}",
            camera_state,
            cam_center.0,
            cam_center.1,
            cam_offset.0,
            cam_offset.1,
            if camera.in_room() { "Y" } else { "n" },
            if camera.near_gap() { "Y" } else { "n" }
        ),
        10.0,
        [0.62, 0.78, 0.98, 1.0]
    ));
    py += lh + 2.0;

    for (i, e) in game.enemies.iter().enumerate() {
        if e.kind == EnemyKind::TargetDummy {
            out.push(ts!(
                px,
                py,
                format!("T{i} [TARGET] hp:{}", e.hp),
                11.0,
                [1.0, 0.85, 0.25, 1.0]
            ));
            py += lh + 3.0;
            continue;
        }

        let state_label = match e.brain.awareness.state {
            game::ai::awareness::AiState::Combat => "COMBAT",
            game::ai::awareness::AiState::Alert => "ALERT ",
            game::ai::awareness::AiState::Idle => "idle  ",
        };
        let sees = if e.brain.awareness.state == game::ai::awareness::AiState::Combat {
            "Y"
        } else {
            "n"
        };
        let line1 = format!(
            "E{i} [{state_label}] susp:{:.2} hp:{} vis:{}",
            e.brain.awareness.suspicion, e.hp, sees
        );
        let col1 = if e.brain.awareness.in_combat() {
            [1.0, 0.35, 0.35, 1.0]
        } else if e.brain.awareness.is_alert() {
            [1.0, 0.65, 0.2, 1.0]
        } else {
            [0.55, 0.8, 0.55, 1.0]
        };
        out.push(ts!(px, py, line1, 11.0, col1));
        py += lh;

        let pos = (e.movement.x, e.movement.y);
        let anchor = e.brain.spawn_anchor();
        out.push(ts!(
            px + 8.0,
            py,
            format!(
                "pos:({:.0},{:.0}) anc:({:.0},{:.0})",
                pos.0, pos.1, anchor.0, anchor.1
            ),
            10.0,
            [0.6, 0.6, 0.6, 1.0]
        ));
        py += lh;

        out.push(ts!(
            px + 8.0,
            py,
            format!("phase:{}", e.brain.phase_name()),
            10.0,
            [0.5, 0.75, 1.0, 1.0]
        ));
        py += lh;

        if let Some(lk) = e.brain.awareness.last_known_pos() {
            out.push(ts!(
                px + 8.0,
                py,
                format!("last_known:({:.0},{:.0})", lk.0, lk.1),
                10.0,
                [0.85, 0.5, 0.85, 1.0]
            ));
            py += lh;
        }

        if let Some(mv) = e.brain.last_move_target {
            out.push(ts!(
                px + 8.0,
                py,
                format!("move_to:({:.0},{:.0})", mv.0, mv.1),
                10.0,
                [0.4, 1.0, 0.6, 1.0]
            ));
            py += lh;
        }

        py += 3.0; // gap between enemies
    }
    out
}

fn editor_texts(sw: f32, sh: f32, editor: &Editor) -> Vec<TextSection> {
    let tool = match editor.tool {
        Tool::PlayerSpawn => "Player Spawn",
        Tool::Enemy => "Enemy",
        Tool::Wall => "Wall (2-point, 0.1 tile)",
        Tool::TargetDummy => "Target Dummy",
        Tool::Breakable => "Breakable Wall (2-point)",
        Tool::Prop => "Prop",
        Tool::BaseMap => "Base Map (2-point bounds)",
    };
    let prop_info = match editor.selected_prop_asset() {
        Some(asset) => format!(
            "Prop Id: {} ({}/{})  {:.2}x{:.2}  collider:{}",
            asset.id,
            editor.selected_prop_asset_index() + 1,
            editor.prop_assets().len(),
            asset.width,
            asset.height,
            if asset.is_collider { "yes" } else { "no" }
        ),
        None => "Prop Id: (none found in assets/props/*.json)".to_string(),
    };
    let assets_line = if editor.prop_assets().is_empty() {
        "Prop Ids: (none)".to_string()
    } else {
        let mut labels: Vec<String> = editor
            .prop_assets()
            .iter()
            .enumerate()
            .take(6)
            .map(|(idx, asset)| {
                if idx == editor.selected_prop_asset_index() {
                    format!("[{}]", asset.id)
                } else {
                    asset.id.clone()
                }
            })
            .collect();
        if editor.prop_assets().len() > 6 {
            labels.push(format!("+{}", editor.prop_assets().len() - 6));
        }
        format!("Prop Ids: {}", labels.join("  "))
    };
    let grid = format!(
        "Snap: {}  Inner Grid: {}",
        editor.active_snap_label(),
        if editor.show_subgrid { "ON" } else { "OFF" }
    );
    let breakables = editor.level.walls.iter().filter(|w| w.breakable).count();
    let solids = editor.level.walls.len().saturating_sub(breakables);
    let stats = format!(
        "Enemies: {}  Targets: {}  Walls: {}  Breakables: {}  Props: {}  Map: {}  Zoom: {:.2}x",
        editor.level.enemies.len(),
        editor.level.target_enemies.len(),
        solids,
        breakables,
        editor.level.props.len(),
        match editor.level.map_bounds {
            Some(b) => format!("{:.1}x{:.1}", b.w, b.h),
            None => "--".to_string(),
        },
        editor.zoom
    );
    vec![
        ts!(8.0, 6.0, "LEVEL EDITOR", 18.0, [1.0, 0.7, 0.2, 1.0]),
        ts!(8.0, 28.0, tool, 15.0, [1.0, 1.0, 1.0, 1.0]),
        ts!(8.0, 46.0, grid, 13.0, [0.6, 0.6, 0.6, 1.0]),
        ts!(8.0, 64.0, prop_info, 13.0, [0.6, 0.7, 0.9, 1.0]),
        ts!(8.0, 82.0, assets_line, 13.0, [0.58, 0.65, 0.86, 1.0]),
        ts!(
            6.0,
            sh - 38.0,
            "1: Spawn   2: Enemy   3: Wall   4: Target   5: Breakable   6: Prop   7: Base Map   Q/E: Prop Id",
            13.0,
            [0.55, 0.55, 0.55, 1.0]
        ),
        ts!(
            6.0,
            sh - 20.0,
            "Left: place   Right: delete   Wheel or +/-: zoom   WASD/Arrows: pan   F5: save   L: load   F1: play   Esc: menu",
            13.0,
            [0.55, 0.55, 0.55, 1.0]
        ),
        ts!(sw - 180.0, 6.0, stats, 13.0, [0.5, 0.5, 0.5, 1.0]),
    ]
}

// ---------------------------------------------------------------------------
// Play-mode quad builder
// ---------------------------------------------------------------------------

fn world_pos_to_screen(
    camera: &TacticalCamera,
    viewport_px: (f32, f32),
    pos: (f32, f32),
) -> (f32, f32) {
    camera.world_to_screen(pos, viewport_px)
}

fn world_points_to_screen(
    camera: &TacticalCamera,
    viewport_px: (f32, f32),
    points: Vec<[f32; 2]>,
) -> Vec<[f32; 2]> {
    points
        .into_iter()
        .map(|p| {
            let s = camera.world_to_screen((p[0], p[1]), viewport_px);
            [s.0, s.1]
        })
        .collect()
}

fn play_quads(
    game: &Game,
    camera: &TacticalCamera,
    debug: bool,
    sw: f32,
    sh: f32,
) -> Vec<QuadInstance> {
    let viewport_px = (sw, sh);
    let mut out = Vec::new();
    if let Some(bounds) = game.level_bounds {
        let center = world_pos_to_screen(
            camera,
            viewport_px,
            (bounds.x + bounds.w * 0.5, bounds.y + bounds.h * 0.5),
        );
        out.push(QuadInstance {
            center: [center.0, center.1],
            half_size: [tiles_to_px(bounds.w * 0.5), tiles_to_px(bounds.h * 0.5)],
            color: [0.07, 0.07, 0.11, 1.0],
        });
    }
    for w in &game.walls {
        if w.breakable && !w.segments.is_empty() {
            let n = w.segments.len();
            for (i, &alive) in w.segments.iter().enumerate() {
                if !alive {
                    continue;
                }
                let (sx, sy, sw, sh) = w.segment_rect(i, n);
                let center =
                    world_pos_to_screen(camera, viewport_px, (sx + sw * 0.5, sy + sh * 0.5));
                out.push(QuadInstance {
                    center: [center.0, center.1],
                    half_size: [tiles_to_px(sw * 0.5), tiles_to_px(sh * 0.5)],
                    color: [0.2, 0.8, 0.95, 1.0],
                });
            }
        } else {
            let center =
                world_pos_to_screen(camera, viewport_px, (w.x + w.w * 0.5, w.y + w.h * 0.5));
            out.push(QuadInstance {
                center: [center.0, center.1],
                half_size: [tiles_to_px(w.w * 0.5), tiles_to_px(w.h * 0.5)],
                color: if w.breakable {
                    [0.2, 0.8, 0.95, 1.0]
                } else {
                    [0.45, 0.4, 0.35, 1.0]
                },
            });
        }
    }
    for prop_instance in &game.props {
        let center = world_pos_to_screen(camera, viewport_px, (prop_instance.x, prop_instance.y));
        out.push(QuadInstance {
            center: [center.0, center.1],
            half_size: [
                tiles_to_px(prop_instance.width * 0.5),
                tiles_to_px(prop_instance.height * 0.5),
            ],
            color: prop::asset_color(&prop_instance.id, prop_instance.is_collider, 1.0),
        });
    }
    let player = world_pos_to_screen(
        camera,
        viewport_px,
        (game.player.movement.x, game.player.movement.y),
    );
    out.push(QuadInstance {
        center: [player.0, player.1],
        half_size: [10.0, 10.0],
        color: [1.0, 1.0, 1.0, 1.0],
    });
    for e in &game.enemies {
        let visible = e.visible_to_player;
        if !visible && !debug {
            continue;
        }
        // Dimmer when not visible to player (debug see-through).
        let color = match (e.kind, visible) {
            (EnemyKind::Shooter, true) => [1.0, 0.2, 0.2, 1.0],
            (EnemyKind::Shooter, false) => [0.6, 0.15, 0.15, 0.55],
            (EnemyKind::TargetDummy, true) => [1.0, 0.85, 0.2, 1.0],
            (EnemyKind::TargetDummy, false) => [0.6, 0.5, 0.12, 0.55],
        };
        let enemy = world_pos_to_screen(camera, viewport_px, (e.movement.x, e.movement.y));
        out.push(QuadInstance {
            center: [enemy.0, enemy.1],
            half_size: [8.0, 8.0],
            color,
        });

        if debug && e.kind == EnemyKind::Shooter {
            let ep = (e.movement.x, e.movement.y);

            // Spawn anchor — blue dot.
            let anchor = e.brain.spawn_anchor();
            let anchor = world_pos_to_screen(camera, viewport_px, anchor);
            out.push(QuadInstance {
                center: [anchor.0, anchor.1],
                half_size: [4.0, 4.0],
                color: [0.3, 0.5, 1.0, 0.8],
            });

            // Last known player position — magenta dot.
            if let Some(lk) = e.brain.awareness.last_known_pos() {
                let lk = world_pos_to_screen(camera, viewport_px, lk);
                out.push(QuadInstance {
                    center: [lk.0, lk.1],
                    half_size: [5.0, 5.0],
                    color: [1.0, 0.3, 1.0, 0.85],
                });
            }

            // Current move target — green dot.
            if let Some(mv) = e.brain.last_move_target {
                let mv = world_pos_to_screen(camera, viewport_px, mv);
                out.push(QuadInstance {
                    center: [mv.0, mv.1],
                    half_size: [4.0, 4.0],
                    color: [0.2, 1.0, 0.4, 0.85],
                });
            }

            // Gap waypoints — cyan dots.
            for gap in e.brain.debug_gaps(ep, &game.walls) {
                let gap = world_pos_to_screen(camera, viewport_px, gap);
                out.push(QuadInstance {
                    center: [gap.0, gap.1],
                    half_size: [4.0, 4.0],
                    color: [0.0, 0.9, 0.9, 0.75],
                });
            }
        }
    }
    for b in &game.bullets {
        let bullet = world_pos_to_screen(camera, viewport_px, (b.x, b.y));
        out.push(QuadInstance {
            center: [bullet.0, bullet.1],
            half_size: [3.0, 3.0],
            color: [1.0, 1.0, 0.0, 1.0],
        });
    }
    out
}

// ---------------------------------------------------------------------------
// Play-mode sight-cone geometry builder
// ---------------------------------------------------------------------------

fn play_geo(game: &Game, camera: &TacticalCamera, debug: bool, sw: f32, sh: f32, net_bullets: &[NetBulletTrace]) -> Vec<GeoVertex> {
    let viewport_px = (sw, sh);
    let mut out = Vec::new();
    let walls = &game.walls;
    let player_pos = (game.player.movement.x, game.player.movement.y);
    let player_pos_px = world_pos_to_screen(camera, viewport_px, player_pos);

    // ── Level room / gap overlay (standalone, no enemy involvement) ───────────
    if debug {
        // Use cached rooms from level load (no per-frame recomputation)
        push_level_rooms_geo(&mut out, &game.rooms, camera, viewport_px);
    }

    // ── Player ────────────────────────────────────────────────────────────────
    let circle = world_points_to_screen(
        camera,
        viewport_px,
        game.player.sight.circle_arc_pts(player_pos, walls, 64),
    );
    push_cone_fan(&mut out, player_pos_px, &circle, [0.3, 0.7, 1.0, 0.07]);
    let arc = world_points_to_screen(
        camera,
        viewport_px,
        game.player.sight.cone_arc_pts(player_pos, walls, 60),
    );
    push_cone_fan(&mut out, player_pos_px, &arc, [0.3, 0.7, 1.0, 0.16]);
    let aim_arc = world_points_to_screen(
        camera,
        viewport_px,
        game.player.aim_cone.cone_arc_pts(player_pos, walls, 16),
    );
    push_cone_fan(&mut out, player_pos_px, &aim_arc, [1.0, 0.6, 0.1, 0.45]);

    // Bullet impact marks.
    for impact in &game.impacts {
        let impact_pos = world_pos_to_screen(camera, viewport_px, (impact.x, impact.y));
        push_circle_fan(
            &mut out,
            impact_pos,
            5.0,
            [1.0, 0.95, 0.2, 0.22 * impact.alpha()],
            18,
        );
    }

    // ── Server-authoritative bullet traces ────────────────────────────────────
    for trace in net_bullets {
        let alpha = (trace.ttl / NET_BULLET_TTL).clamp(0.0, 1.0);
        let from = world_pos_to_screen(camera, viewport_px, (trace.from_x, trace.from_y));
        let mut to = world_pos_to_screen(camera, viewport_px, (trace.to_x, trace.to_y));

        // Clamp the endpoint so the trace stays on-screen even when the server
        // reports a hit point far outside the level (e.g. no walls loaded).
        let dx = to.0 - from.0;
        let dy = to.1 - from.1;
        let dist = (dx * dx + dy * dy).sqrt();
        if dist > NET_BULLET_MAX_SCREEN_PX {
            let s = NET_BULLET_MAX_SCREEN_PX / dist;
            to = (from.0 + dx * s, from.1 + dy * s);
        }

        push_line(&mut out, from, to, 3.0, [1.0, 0.9, 0.3, 0.9 * alpha]);
        push_circle_fan(&mut out, to, 5.0, [1.0, 0.85, 0.2, 0.5 * alpha], 12);
    }

    // ── Enemies ───────────────────────────────────────────────────────────────
    for e in &game.enemies {
        if e.kind == EnemyKind::TargetDummy {
            continue;
        }
        let visible = e.visible_to_player;
        if !visible && !debug {
            continue;
        }
        let ep = (e.movement.x, e.movement.y);
        let ep_px = world_pos_to_screen(camera, viewport_px, ep);

        // Nearby circle — dimmed when not visible to player.
        let circle_alpha = if visible { 0.05 } else { 0.03 };
        push_circle_fan(
            &mut out,
            ep_px,
            tiles_to_px(e.sight.circle_radius),
            [1.0, 0.3, 0.3, circle_alpha],
            36,
        );

        // Cone colour: red=combat, orange=alert, yellow=idle. Dimmed if not visible.
        let alpha_scale = if visible { 1.0 } else { 0.45 };
        let cone_color = if e.brain.awareness.in_combat() {
            [1.0, 0.08, 0.08, 0.35 * alpha_scale]
        } else if e.brain.awareness.is_alert() {
            [1.0, 0.50, 0.05, 0.28 * alpha_scale]
        } else {
            [1.0, 0.85, 0.20, 0.14 * alpha_scale]
        };
        let arc = world_points_to_screen(camera, viewport_px, e.sight.cone_arc_pts(ep, walls, 48));
        push_cone_fan(&mut out, ep_px, &arc, cone_color);
    }

    out
}

/// Render the level's room / gap structure derived from `LevelRooms`:
///  - Each scan point's 36-sector polygon is tinted with the room's colour.
/// Draw detected rooms and gaps:
///  - Each room is drawn as a semi-transparent rectangle with its assigned color.
///  - Deduplicated gap waypoints are drawn as small filled diamonds.
fn push_level_rooms_geo(
    out: &mut Vec<GeoVertex>,
    rooms: &LevelRooms,
    camera: &TacticalCamera,
    viewport_px: (f32, f32),
) {
    // Draw each detected room as a rectangle.
    for room in &rooms.rooms {
        let color = room.color;
        let (x, y) = world_pos_to_screen(camera, viewport_px, (room.x, room.y));
        let (x2, y2) = world_pos_to_screen(camera, viewport_px, (room.x + room.w, room.y + room.h));

        // Two triangles to fill the rectangle.
        out.push(GeoVertex { pos: [x, y], color });
        out.push(GeoVertex {
            pos: [x2, y],
            color,
        });
        out.push(GeoVertex {
            pos: [x, y2],
            color,
        });

        out.push(GeoVertex {
            pos: [x2, y],
            color,
        });
        out.push(GeoVertex {
            pos: [x2, y2],
            color,
        });
        out.push(GeoVertex {
            pos: [x, y2],
            color,
        });
    }

    // Gap waypoints: small filled diamonds (two triangles).
    let gap_col = [1.0, 0.75, 0.05, 0.90];
    const R: f32 = 5.0;
    for &(gx, gy) in &rooms.gaps {
        let (gx, gy) = world_pos_to_screen(camera, viewport_px, (gx, gy));
        // Diamond = 4 points (N, E, S, W) → 2 triangles.
        let n_pt = [gx, gy - R];
        let e_pt = [gx + R, gy];
        let s_pt = [gx, gy + R];
        let w_pt = [gx - R, gy];
        out.push(GeoVertex {
            pos: n_pt,
            color: gap_col,
        });
        out.push(GeoVertex {
            pos: e_pt,
            color: gap_col,
        });
        out.push(GeoVertex {
            pos: s_pt,
            color: gap_col,
        });
        out.push(GeoVertex {
            pos: n_pt,
            color: gap_col,
        });
        out.push(GeoVertex {
            pos: s_pt,
            color: gap_col,
        });
        out.push(GeoVertex {
            pos: w_pt,
            color: gap_col,
        });
    }
}

fn enemies_in_combat(enemies: &[game::entity::enemy::Enemy]) -> bool {
    enemies
        .iter()
        .any(|enemy| enemy.kind == EnemyKind::Shooter && enemy.brain.awareness.in_combat())
}

fn infer_world_bounds(game: &Game, fallback_w: f32, fallback_h: f32) -> CameraBounds {
    if let Some(bounds) = game.level_bounds {
        // Give camera headroom past authored map bounds so edge movement
        // doesn't feel like a hard sticky clamp.
        const MAP_BOUNDS_CAMERA_PADDING: f32 = px_to_tiles(256.0);
        return CameraBounds::from_min_max(
            bounds.x - MAP_BOUNDS_CAMERA_PADDING,
            bounds.y - MAP_BOUNDS_CAMERA_PADDING,
            bounds.x + bounds.w + MAP_BOUNDS_CAMERA_PADDING,
            bounds.y + bounds.h + MAP_BOUNDS_CAMERA_PADDING,
        );
    }

    const EDGE_PADDING: f32 = px_to_tiles(128.0);
    let mut min_x = 0.0_f32;
    let mut min_y = 0.0_f32;
    let mut max_x = fallback_w.max(game.player.movement.x);
    let mut max_y = fallback_h.max(game.player.movement.y);

    let mut include_point = |x: f32, y: f32| {
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    };

    include_point(game.player.movement.x, game.player.movement.y);
    for enemy in &game.enemies {
        include_point(enemy.movement.x, enemy.movement.y);
    }
    for wall in &game.walls {
        include_point(wall.x, wall.y);
        include_point(wall.x + wall.w, wall.y + wall.h);
    }
    for prop_instance in &game.props {
        include_point(
            prop_instance.x - prop_instance.width * 0.5,
            prop_instance.y - prop_instance.height * 0.5,
        );
        include_point(
            prop_instance.x + prop_instance.width * 0.5,
            prop_instance.y + prop_instance.height * 0.5,
        );
    }

    min_x = (min_x - EDGE_PADDING).min(0.0);
    min_y = (min_y - EDGE_PADDING).min(0.0);
    max_x = (max_x + EDGE_PADDING).max(fallback_w);
    max_y = (max_y + EDGE_PADDING).max(fallback_h);
    CameraBounds::from_min_max(min_x, min_y, max_x, max_y)
}

/// Draw a filled rectangle between two screen-space points `a` and `b` with
/// the given pixel `width`.  The rectangle is decomposed into two triangles.
fn push_line(out: &mut Vec<GeoVertex>, a: (f32, f32), b: (f32, f32), width: f32, color: [f32; 4]) {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 0.5 {
        return;
    }
    let half = width * 0.5;
    let nx = -dy / len * half;
    let ny =  dx / len * half;

    let v0 = [a.0 + nx, a.1 + ny];
    let v1 = [a.0 - nx, a.1 - ny];
    let v2 = [b.0 + nx, b.1 + ny];
    let v3 = [b.0 - nx, b.1 - ny];

    // Triangle 1
    out.push(GeoVertex { pos: v0, color });
    out.push(GeoVertex { pos: v1, color });
    out.push(GeoVertex { pos: v2, color });
    // Triangle 2
    out.push(GeoVertex { pos: v1, color });
    out.push(GeoVertex { pos: v3, color });
    out.push(GeoVertex { pos: v2, color });
}

fn client_time_ms() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u32
}

fn scan_first_level() -> Option<LevelData> {
    let dir = std::fs::read_dir("assets/levels").ok()?;
    let mut paths: Vec<_> = dir
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("json"))
        .collect();
    paths.sort();
    paths.first().and_then(|p| p.to_str()).and_then(|s| LevelData::load(s).ok())
}
