use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use engine::input::InputState;
use game::game::Game;
use game::world::level::{LevelBounds, LevelData};
use game::world::sight::Sight;
use game::world::units::{px_to_tiles, tiles_to_px};
use game::world::wall::{self, Wall};
use net::proto::server::{
    BulletEvent, MatchState, MovementState, PlayerState, SelfState, ServerPacket,
};
use net::{
    AnyPacket, ClientPacket, ConnectAck, ConnectResult, LobbyCommandKind, LobbyState, MoveSpeed,
    NetSocket, PROTOCOL_VERSION, decode_rotation, encode, encode_rotation,
};
use tokio::time::{Instant as TokioInstant, sleep_until};

const TICK_RATE: u64 = 60;
const TICK_DURATION: Duration = Duration::from_nanos(1_000_000_000 / TICK_RATE);
const UDP_PORT: u16 = 7777;
const MAX_PLAYERS: usize = 16;
const PLAYER_HALF: f32 = px_to_tiles(10.0);
const WALK_SPEED: f32 = px_to_tiles(40.0);
const NORMAL_SPEED: f32 = px_to_tiles(85.0);
const RUN_SPEED: f32 = px_to_tiles(200.0);

// ── Session ───────────────────────────────────────────────────────────────────

struct Session {
    player_id: u16,
    name: String,
    x: f32,
    y: f32,
    rotation: u16,
    movement_state: MovementState,
    /// Team selection (`0` = none, `1` = team 1, `2` = team 2).
    team: u8,
    /// Latest input received from this client (updated every tick).
    latest_input: Option<ClientPacket>,
}

struct ServerState {
    sessions: HashMap<SocketAddr, Session>,
    next_player_id: u16,
    game_started: bool,
    spawn: (f32, f32),
}

impl ServerState {
    fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            next_player_id: 1,
            game_started: false,
            spawn: (px_to_tiles(400.0), px_to_tiles(300.0)),
        }
    }
}

type Shared = Arc<Mutex<ServerState>>;

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let socket: Arc<NetSocket> = Arc::new(
        NetSocket::bind(format!("0.0.0.0:{UDP_PORT}").parse().unwrap())
            .await
            .expect("failed to bind UDP socket"),
    );

    let state: Shared = Arc::new(Mutex::new(ServerState::new()));

    let recv_socket = Arc::clone(&socket);
    let recv_state = Arc::clone(&state);
    tokio::spawn(async move {
        recv_loop(recv_socket, recv_state).await;
    });

    let mut game = Game::new();
    // Load the first available level so the server's player spawn, walls, and
    // map bounds match what the client has loaded.  Without this, bullet traces
    // are computed at the server's default spawn (≈ 0,0 in px) while the
    // client's camera is centred on the level spawn — putting every trace
    // hundreds of pixels off-screen.
    if let Some(level) = load_first_level() {
        let level_w = level.map_bounds.map_or(0.0, |b| b.x + b.w);
        let level_h = level.map_bounds.map_or(0.0, |b| b.y + b.h);
        game.load_level(&level, level_w, level_h);
        {
            let mut st = state.lock().unwrap();
            st.spawn = (game.player.movement.x, game.player.movement.y);
        }
        println!(
            "[server] loaded level '{}' (spawn={:.2},{:.2})",
            level.id,
            level.player_spawn.map_or(0.0, |s| s.x),
            level.player_spawn.map_or(0.0, |s| s.y)
        );
    } else {
        println!("[server] no level found, using default spawn");
    }
    let mut tick: u32 = 0;
    let mut next_tick = TokioInstant::now();

    loop {
        sleep_until(next_tick).await;

        // Advance all connected player replicas and collect latest inputs.
        let (game_started, addrs, player_input): (
            bool,
            Vec<SocketAddr>,
            Option<(u16, ClientPacket)>,
        ) = {
            let mut st = state.lock().unwrap();
            for session in st.sessions.values_mut() {
                if let Some(input) = session.latest_input {
                    apply_session_input(
                        session,
                        &input,
                        TICK_DURATION.as_secs_f32(),
                        &game.walls,
                        game.level_bounds,
                    );
                }
            }
            let addrs = st.sessions.keys().copied().collect();
            // For now advance the game with the first connected player's input.
            let input = st
                .sessions
                .values()
                .filter_map(|s| s.latest_input.map(|i| (s.player_id, i)))
                .next();
            (st.game_started, addrs, input)
        };

        if !game_started {
            tick = tick.wrapping_add(1);
            next_tick += TICK_DURATION;
            if TokioInstant::now() > next_tick {
                next_tick = TokioInstant::now() + TICK_DURATION;
            }
            continue;
        }

        // Convert the client packet into an engine InputState and step the game.
        let engine_input = player_input
            .map(|(_, pkt)| {
                client_input_to_engine(&pkt, game.player.movement.x, game.player.movement.y)
            })
            .unwrap_or_default();

        game.update(TICK_DURATION.as_secs_f32(), &engine_input);

        // The shooter_id for all traces this tick is the first connected player
        // (or 0 if nobody is connected yet).
        let shooter_id = player_input.map(|(id, _)| id).unwrap_or(0);

        // Drain bullet traces and convert to wire events.
        let bullets: Vec<BulletEvent> = game
            .take_bullet_traces()
            .into_iter()
            .map(|t| BulletEvent {
                shooter_id,
                from_x: t.origin_x,
                from_y: t.origin_y,
                to_x: t.x,
                to_y: t.y,
                hit_player_id: 0,
            })
            .collect();

        tick = tick.wrapping_add(1);
        next_tick += TICK_DURATION;

        if !addrs.is_empty() {
            let payloads: Vec<(SocketAddr, Vec<u8>)> = {
                let st = state.lock().unwrap();
                st.sessions
                    .iter()
                    .map(|(&addr, session)| {
                        let snapshot = build_snapshot_for_player(
                            tick,
                            &bullets,
                            &game,
                            &st,
                            addr,
                            session.latest_input.as_ref(),
                        );
                        (addr, encode(&AnyPacket::ServerSnapshot(snapshot)))
                    })
                    .collect()
            };
            for (addr, payload) in payloads {
                let _ = socket.send_raw(&payload, addr).await;
            }
        }

        if TokioInstant::now() > next_tick {
            next_tick = TokioInstant::now() + TICK_DURATION;
        }
    }
}

// ── Receive loop ──────────────────────────────────────────────────────────────

async fn recv_loop(socket: Arc<NetSocket>, state: Shared) {
    loop {
        match socket.recv().await {
            Ok((packet, addr)) => handle_packet(packet, addr, &socket, &state).await,
            Err(_) => {}
        }
    }
}

async fn handle_packet(packet: AnyPacket, addr: SocketAddr, socket: &NetSocket, state: &Shared) {
    match packet {
        AnyPacket::ConnectRequest(req) => {
            enum Decision {
                ReAck(u16),
                NewPlayer(u16),
                Full,
                MatchStarted,
                VersionMismatch,
            }

            let decision = {
                let mut st = state.lock().unwrap();
                if let Some(s) = st.sessions.get(&addr) {
                    Decision::ReAck(s.player_id)
                } else if req.version != PROTOCOL_VERSION {
                    Decision::VersionMismatch
                } else if st.game_started {
                    Decision::MatchStarted
                } else if st.sessions.len() >= MAX_PLAYERS {
                    Decision::Full
                } else {
                    let pid = st.next_player_id;
                    st.next_player_id = pid.wrapping_add(1).max(1);
                    let spawn = st.spawn;
                    st.sessions.insert(
                        addr,
                        Session {
                            player_id: pid,
                            name: req.name_str().to_string(),
                            x: spawn.0,
                            y: spawn.1,
                            rotation: encode_rotation(0.0),
                            movement_state: MovementState::default(),
                            team: 0,
                            latest_input: None,
                        },
                    );
                    Decision::NewPlayer(pid)
                }
            };

            match decision {
                Decision::Full => {
                    let _ = socket
                        .send_to(
                            &AnyPacket::ConnectAck(ConnectAck {
                                result: ConnectResult::ServerFull,
                                player_id: 0,
                                tick_rate: TICK_RATE as u8,
                                server_time: server_time_ms(),
                            }),
                            addr,
                        )
                        .await;
                }
                Decision::MatchStarted => {
                    let _ = socket
                        .send_to(
                            &AnyPacket::ConnectAck(ConnectAck {
                                result: ConnectResult::MatchStarted,
                                player_id: 0,
                                tick_rate: TICK_RATE as u8,
                                server_time: server_time_ms(),
                            }),
                            addr,
                        )
                        .await;
                }
                Decision::VersionMismatch => {
                    let _ = socket
                        .send_to(
                            &AnyPacket::ConnectAck(ConnectAck {
                                result: ConnectResult::VersionMismatch,
                                player_id: 0,
                                tick_rate: TICK_RATE as u8,
                                server_time: server_time_ms(),
                            }),
                            addr,
                        )
                        .await;
                }
                Decision::ReAck(pid) | Decision::NewPlayer(pid) => {
                    let _ = socket
                        .accept(&req, addr, pid, TICK_RATE as u8, server_time_ms())
                        .await;
                    broadcast_lobby_state(socket, state).await;
                }
            }
        }

        AnyPacket::Disconnect => {
            state.lock().unwrap().sessions.remove(&addr);
            broadcast_lobby_state(socket, state).await;
        }

        AnyPacket::ClientInput(input) => {
            if let Some(session) = state.lock().unwrap().sessions.get_mut(&addr) {
                session.latest_input = Some(input);
            }
        }

        AnyPacket::LobbyCommand(cmd) => {
            let mut should_broadcast = false;
            {
                let mut st = state.lock().unwrap();
                if !st.game_started {
                    match cmd.kind {
                        LobbyCommandKind::SelectTeam => {
                            if let Some(session) = st.sessions.get_mut(&addr) {
                                session.team = if cmd.team == 2 { 2 } else { 1 };
                                should_broadcast = true;
                            }
                        }
                        LobbyCommandKind::StartGame => {
                            if st.sessions.contains_key(&addr) {
                                st.game_started = true;
                                should_broadcast = true;
                            }
                        }
                    }
                }
            }
            if should_broadcast {
                broadcast_lobby_state(socket, state).await;
            }
        }

        // Ping is a low-level protocol concern handled by NetSocket; ignore here
        AnyPacket::Ping(ts) => {
            let _ = socket.send_to(&AnyPacket::Pong(ts), addr).await;
        }

        _ => {}
    }
}

fn lobby_state_for(st: &ServerState, addr: SocketAddr) -> LobbyState {
    let mut team1 = 0u8;
    let mut team2 = 0u8;
    for session in st.sessions.values() {
        if session.team == 1 {
            team1 = team1.saturating_add(1);
        } else if session.team == 2 {
            team2 = team2.saturating_add(1);
        }
    }

    LobbyState {
        game_started: st.game_started,
        your_team: st.sessions.get(&addr).map(|s| s.team).unwrap_or(0),
        team1_count: team1,
        team2_count: team2,
    }
}

async fn broadcast_lobby_state(socket: &NetSocket, state: &Shared) {
    let per_client_packets: Vec<(SocketAddr, Vec<u8>)> = {
        let st = state.lock().unwrap();
        st.sessions
            .keys()
            .copied()
            .map(|addr| {
                let payload = encode(&AnyPacket::LobbyState(lobby_state_for(&st, addr)));
                (addr, payload)
            })
            .collect()
    };

    for (addr, payload) in per_client_packets {
        let _ = socket.send_raw(&payload, addr).await;
    }
}

// ── Input conversion ──────────────────────────────────────────────────────────

/// Convert a `ClientPacket` into an `InputState` the game engine can consume.
///
/// The rotation angle is decoded back to a direction vector, and a fake
/// mouse-world position one tile ahead of the player is synthesised so that
/// `Player::update` computes the correct aim direction.
fn client_input_to_engine(pkt: &ClientPacket, player_x: f32, player_y: f32) -> InputState {
    let theta = decode_rotation(pkt.rotation);
    let dir_x = theta.cos();
    let dir_y = theta.sin();

    // One tile ahead in the aim direction, expressed as pixel coordinates
    // (the engine converts pixels → tiles internally via px_to_tiles).
    let aim_px_x = tiles_to_px(player_x + dir_x) as f64;
    let aim_px_y = tiles_to_px(player_y + dir_y) as f64;

    InputState {
        right: pkt.movement_x > 0,
        left: pkt.movement_x < 0,
        down: pkt.movement_y > 0,
        up: pkt.movement_y < 0,
        shoot: pkt.flags.is_shooting(),
        reload: pkt.flags.is_reloading(),
        walk: pkt.flags.move_speed() == MoveSpeed::SlowWalk,
        shift: pkt.flags.move_speed() == MoveSpeed::Run,
        peek: pkt.flags.is_peeking(),
        mouse_x: aim_px_x,
        mouse_y: aim_px_y,
        ..InputState::default()
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn build_snapshot_for_player(
    tick: u32,
    bullets: &[BulletEvent],
    game: &Game,
    st: &ServerState,
    recipient_addr: SocketAddr,
    latest_input: Option<&ClientPacket>,
) -> ServerPacket {
    let fallback_rotation = encode_rotation(game.player.sight.direction);
    let (me_x, me_y, me_rotation, me_movement_state) =
        if let Some(session) = st.sessions.get(&recipient_addr) {
            (
                session.x,
                session.y,
                session.rotation,
                session.movement_state,
            )
        } else {
            (
                game.player.movement.x,
                game.player.movement.y,
                fallback_rotation,
                movement_state_from_input(latest_input, game.player.is_reloading()),
            )
        };

    let mut viewer_sight = Sight::player();
    viewer_sight.direction = decode_rotation(me_rotation);
    viewer_sight.half_angle = game.player.sight.half_angle;
    viewer_sight.range = game.player.sight.range;
    viewer_sight.circle_radius = game.player.sight.circle_radius;
    let viewer_pos = (me_x, me_y);
    let players: Vec<PlayerState> = st
        .sessions
        .iter()
        .filter_map(|(&addr, session)| {
            if addr == recipient_addr {
                return None;
            }
            let target = (session.x, session.y);
            if !viewer_sight.can_see(viewer_pos, target, &game.walls) {
                return None;
            }
            Some(PlayerState {
                id: session.player_id,
                x: session.x,
                y: session.y,
                rotation: session.rotation,
                movement_state: session.movement_state,
                weapon: 0,
            })
        })
        .collect();

    ServerPacket {
        tick,
        timestamp: latest_input.map(|pkt| pkt.timestamp).unwrap_or(0),
        me: SelfState {
            health: 100,
            ammo: game.player.ammo_in_mag() as u8,
            // Non-zero while reloading (exact progress tracked client-side).
            reload_progress: if game.player.is_reloading() { 128 } else { 0 },
            movement_state: me_movement_state,
            x: me_x,
            y: me_y,
            rotation: me_rotation,
            aim_cone_half_angle: game.player.aim_cone.half_angle(),
        },
        players,
        bullets: bullets.to_vec(),
        sounds: vec![],
        match_state: MatchState {
            timer: 300,
            score_team1: 0,
            score_team2: 0,
        },
    }
}

fn apply_session_input(
    session: &mut Session,
    input: &ClientPacket,
    dt: f32,
    walls: &[Wall],
    level_bounds: Option<LevelBounds>,
) {
    let speed = player_speed_from_input(input);
    let mut dx = input.movement_x as f32;
    let mut dy = input.movement_y as f32;
    let len = (dx * dx + dy * dy).sqrt();
    if len > 1.0 {
        dx /= len;
        dy /= len;
    }

    session.x += dx * speed * dt;
    session.y += dy * speed * dt;
    wall::resolve_all(&mut session.x, &mut session.y, PLAYER_HALF, walls);
    clamp_actor_to_level_bounds(&mut session.x, &mut session.y, PLAYER_HALF, level_bounds);
    session.rotation = input.rotation;
    session.movement_state = movement_state_from_input(Some(input), false);
}

fn player_speed_from_input(input: &ClientPacket) -> f32 {
    match input.flags.move_speed() {
        MoveSpeed::SlowWalk => WALK_SPEED,
        MoveSpeed::Walk => NORMAL_SPEED,
        MoveSpeed::Run => RUN_SPEED,
    }
}

fn movement_state_from_input(
    latest_input: Option<&ClientPacket>,
    is_reloading: bool,
) -> MovementState {
    let mut state = MovementState(0);
    state.set_move_speed(
        latest_input
            .map(|pkt| pkt.flags.move_speed())
            .unwrap_or(MoveSpeed::Walk),
    );
    state.set_peeking(
        latest_input
            .map(|pkt| pkt.flags.is_peeking())
            .unwrap_or(false),
    );
    state.set_reloading(is_reloading);
    state
}

fn clamp_actor_to_level_bounds(x: &mut f32, y: &mut f32, half: f32, bounds: Option<LevelBounds>) {
    let Some(bounds) = bounds else {
        return;
    };
    let min_x = bounds.x + half;
    let max_x = bounds.x + bounds.w - half;
    let min_y = bounds.y + half;
    let max_y = bounds.y + bounds.h - half;

    if min_x <= max_x {
        *x = x.clamp(min_x, max_x);
    } else {
        *x = bounds.x + bounds.w * 0.5;
    }
    if min_y <= max_y {
        *y = y.clamp(min_y, max_y);
    } else {
        *y = bounds.y + bounds.h * 0.5;
    }
}

fn server_time_ms() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u32
}

/// Load the first level file from `assets/levels/`, sorted alphabetically —
/// mirrors the same logic used by the client so both start with identical
/// player spawn, walls, and map bounds.
fn load_first_level() -> Option<LevelData> {
    let dir = std::fs::read_dir("assets/levels").ok()?;
    let mut paths: Vec<_> = dir
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("json"))
        .collect();
    paths.sort();
    paths
        .first()
        .and_then(|p| p.to_str())
        .and_then(|s| LevelData::load(s).ok())
}
