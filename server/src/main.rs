use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use engine::input::InputState;
use game::game::Game;
use game::world::level::LevelData;
use game::world::units::tiles_to_px;
use net::proto::server::{BulletEvent, MatchState, SelfState, ServerPacket};
use net::{
    AnyPacket, ClientPacket, ConnectAck, ConnectResult, MoveSpeed, NetSocket, decode_rotation,
    encode, encode_rotation,
};
use tokio::time::{Instant as TokioInstant, sleep_until};

const TICK_RATE: u64 = 60;
const TICK_DURATION: Duration = Duration::from_nanos(1_000_000_000 / TICK_RATE);
const UDP_PORT: u16 = 7777;
const MAX_PLAYERS: usize = 16;

// ── Session ───────────────────────────────────────────────────────────────────

struct Session {
    player_id: u16,
    name: String,
    /// Latest input received from this client (updated every tick).
    latest_input: Option<ClientPacket>,
}

struct ServerState {
    sessions: HashMap<SocketAddr, Session>,
    next_player_id: u16,
}

impl ServerState {
    fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            next_player_id: 1,
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

        // Collect the latest inputs from all connected sessions.
        let (addrs, player_input): (Vec<SocketAddr>, Option<(u16, ClientPacket)>) = {
            let st = state.lock().unwrap();
            let addrs = st.sessions.keys().copied().collect();
            // For now advance the game with the first connected player's input.
            let input = st
                .sessions
                .values()
                .filter_map(|s| s.latest_input.map(|i| (s.player_id, i)))
                .next();
            (addrs, input)
        };

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
            let latest_input = player_input.map(|(_, pkt)| pkt);
            let snapshot = build_snapshot(tick, bullets, &game, latest_input.as_ref());
            println!(
                "[broadcast] tick={} clients={} bullets={} players={} sounds={}",
                snapshot.tick,
                addrs.len(),
                snapshot.bullets.len(),
                snapshot.players.len(),
                snapshot.sounds.len(),
            );
            for b in &snapshot.bullets {
                println!(
                    "[broadcast]   bullet shooter={} ({:.2},{:.2}) -> ({:.2},{:.2}) hit={}",
                    b.shooter_id, b.from_x, b.from_y, b.to_x, b.to_y, b.hit_player_id,
                );
            }
            let encoded = encode(&AnyPacket::ServerSnapshot(snapshot));
            for addr in &addrs {
                let _ = socket.send_raw(&encoded, *addr).await;
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
            }

            let decision = {
                let mut st = state.lock().unwrap();
                if let Some(s) = st.sessions.get(&addr) {
                    Decision::ReAck(s.player_id)
                } else if st.sessions.len() >= MAX_PLAYERS {
                    Decision::Full
                } else {
                    let pid = st.next_player_id;
                    st.next_player_id = pid.wrapping_add(1).max(1);
                    st.sessions.insert(
                        addr,
                        Session {
                            player_id: pid,
                            name: req.name_str().to_string(),
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
                Decision::ReAck(pid) | Decision::NewPlayer(pid) => {
                    let _ = socket
                        .accept(&req, addr, pid, TICK_RATE as u8, server_time_ms())
                        .await;
                }
            }
        }

        AnyPacket::Disconnect => {
            state.lock().unwrap().sessions.remove(&addr);
        }

        AnyPacket::ClientInput(input) => {
            if let Some(session) = state.lock().unwrap().sessions.get_mut(&addr) {
                session.latest_input = Some(input);
            }
        }

        // Ping is a low-level protocol concern handled by NetSocket; ignore here
        AnyPacket::Ping(ts) => {
            let _ = socket.send_to(&AnyPacket::Pong(ts), addr).await;
        }

        _ => {}
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

fn build_snapshot(
    tick: u32,
    bullets: Vec<BulletEvent>,
    game: &Game,
    latest_input: Option<&ClientPacket>,
) -> ServerPacket {
    let movement_state = movement_state_from_input(latest_input, game.player.is_reloading());
    ServerPacket {
        tick,
        timestamp: latest_input.map(|pkt| pkt.timestamp).unwrap_or(0),
        me: SelfState {
            health: 100,
            ammo: game.player.ammo_in_mag() as u8,
            // Non-zero while reloading (exact progress tracked client-side).
            reload_progress: if game.player.is_reloading() { 128 } else { 0 },
            movement_state,
            x: game.player.movement.x,
            y: game.player.movement.y,
            rotation: encode_rotation(game.player.sight.direction),
            aim_cone_half_angle: game.player.aim_cone.half_angle(),
        },
        players: vec![],
        bullets,
        sounds: vec![],
        match_state: MatchState {
            timer: 300,
            score_team1: 0,
            score_team2: 0,
        },
    }
}

fn movement_state_from_input(
    latest_input: Option<&ClientPacket>,
    is_reloading: bool,
) -> net::MovementState {
    let mut state = net::MovementState(0);
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
