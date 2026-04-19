mod network;
mod session;
mod systems;
mod util;

use std::io::ErrorKind;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use game::game::Game;
use net::{AnyPacket, NetSocket, encode};
use session::{ServerState, Shared};
use tokio::time::{Instant as TokioInstant, sleep_until};

use network::recv::recv_loop;
use systems::{
    combat::step_combat, movement::apply_session_input, rounds::step_rounds,
    snapshot::build_snapshot_for_player,
};
use util::load_first_level;

// ---------------------------------------------------------------------------
// Server constants
// ---------------------------------------------------------------------------

const TICK_RATE: u64 = 60;
const TICK_DURATION: Duration = Duration::from_nanos(1_000_000_000 / TICK_RATE);
const UDP_PORT: u16 = 7777;
const UDP_PORT_ENV: &str = "SERVER_UDP_PORT";
const UDP_BIND_ADDR_ENV: &str = "SERVER_BIND_ADDR";

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    let bind_addr = match resolve_bind_addr() {
        Ok(addr) => addr,
        Err(err) => {
            eprintln!("[server] {err}");
            std::process::exit(2);
        }
    };
    let socket: Arc<NetSocket> = match NetSocket::bind(bind_addr).await {
        Ok(sock) => Arc::new(sock),
        Err(err) if err.kind() == ErrorKind::AddrInUse => {
            eprintln!("[server] failed to bind UDP socket at {bind_addr}: address already in use");
            eprintln!(
                "[server] stop the running server process or set {UDP_BIND_ADDR_ENV}/{UDP_PORT_ENV} to a different address."
            );
            std::process::exit(1);
        }
        Err(err) => {
            eprintln!("[server] failed to bind UDP socket at {bind_addr}: {err}");
            std::process::exit(1);
        }
    };
    if let Ok(local_addr) = socket.local_addr() {
        println!("[server] listening on {local_addr}");
    }

    let state: Shared = Arc::new(Mutex::new(ServerState::new()));

    // Spawn the receive task on a dedicated async worker — it never blocks.
    {
        let recv_socket = Arc::clone(&socket);
        let recv_state = Arc::clone(&state);
        tokio::spawn(async move {
            recv_loop(recv_socket, recv_state).await;
        });
    }

    // Load the first available level so server walls, spawn, and map bounds
    // match the client exactly.
    let mut game = Game::new();
    if let Some(level) = load_first_level() {
        let level_w = level.map_bounds.map_or(0.0, |b| b.x + b.w);
        let level_h = level.map_bounds.map_or(0.0, |b| b.y + b.h);
        game.load_level(&level, level_w, level_h);
        {
            let mut st = state.lock().unwrap();
            st.spawn = (game.player.movement.x, game.player.movement.y);
            st.team1_spawn = level.team1_spawn.map(|p| (p.x, p.y));
            st.team2_spawn = level.team2_spawn.map(|p| (p.x, p.y));
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

    tick_loop(socket, state, game).await;
}

fn resolve_bind_addr() -> Result<SocketAddr, String> {
    if let Ok(raw_addr) = std::env::var(UDP_BIND_ADDR_ENV) {
        return raw_addr
            .parse()
            .map_err(|e| format!("invalid {UDP_BIND_ADDR_ENV}='{raw_addr}': {e}"));
    }

    let port = match std::env::var(UDP_PORT_ENV) {
        Ok(raw_port) => raw_port
            .parse::<u16>()
            .map_err(|e| format!("invalid {UDP_PORT_ENV}='{raw_port}': {e}"))?,
        Err(std::env::VarError::NotPresent) => UDP_PORT,
        Err(std::env::VarError::NotUnicode(_)) => {
            return Err(format!("{UDP_PORT_ENV} contains non-unicode bytes"));
        }
    };

    Ok(SocketAddr::from(([0, 0, 0, 0], port)))
}

// ---------------------------------------------------------------------------
// Tick loop
// ---------------------------------------------------------------------------

async fn tick_loop(socket: Arc<NetSocket>, state: Shared, mut game: Game) {
    let mut tick: u32 = 0;
    let mut next_tick = TokioInstant::now();

    loop {
        sleep_until(next_tick).await;

        // Evict players who have been silent for more than 60 seconds.
        {
            let mut st = state.lock().unwrap();
            let now = std::time::Instant::now();
            st.sessions
                .retain(|_, s| now.duration_since(s.last_seen).as_secs() < 60);
            if st.sessions.is_empty() {
                st.reset_room();
            }
        }

        // Apply latest input for every session.
        // Dead players (health == 0) still get input processed for spectator movement.
        let (game_started, addrs): (bool, Vec<SocketAddr>) = {
            let mut st = state.lock().unwrap();
            for session in st.sessions.values_mut() {
                // Pure mid-game spectators with no team skip movement processing.
                if session.is_spectator && session.team == 0 {
                    continue;
                }
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
            (st.game_started, addrs)
        };

        // Combat and round logic are only active once the match has started.
        let bullets = if game_started {
            let dt = TICK_DURATION.as_secs_f32();
            let mut st = state.lock().unwrap();
            let b = step_combat(&mut st, &mut game.walls, dt);
            step_rounds(&mut st, dt);
            b
        } else {
            vec![]
        };

        tick = tick.wrapping_add(1);
        next_tick += TICK_DURATION;

        // Build and broadcast per-client snapshots.
        if !addrs.is_empty() {
            let payloads: Vec<(SocketAddr, Vec<u8>)> = {
                let st = state.lock().unwrap();
                st.sessions
                    .iter()
                    .map(|(&addr, session)| {
                        let snapshot = build_snapshot_for_player(
                            tick,
                            &bullets,
                            &game.walls,
                            &st,
                            addr,
                            session,
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
