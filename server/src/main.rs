mod network;
mod session;
mod systems;
mod util;

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use game::game::Game;
use net::{AnyPacket, NetSocket, encode};
use session::{ServerState, Shared};
use tokio::time::{Instant as TokioInstant, sleep_until};

use network::recv::recv_loop;
use systems::{
    combat::step_combat, movement::apply_session_input, snapshot::build_snapshot_for_player,
};
use util::load_first_level;

// ---------------------------------------------------------------------------
// Server constants
// ---------------------------------------------------------------------------

const TICK_RATE: u64 = 60;
const TICK_DURATION: Duration = Duration::from_nanos(1_000_000_000 / TICK_RATE);
const UDP_PORT: u16 = 7777;

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    let socket: Arc<NetSocket> = Arc::new(
        NetSocket::bind(format!("0.0.0.0:{UDP_PORT}").parse().unwrap())
            .await
            .expect("failed to bind UDP socket"),
    );

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

// ---------------------------------------------------------------------------
// Tick loop
// ---------------------------------------------------------------------------

async fn tick_loop(socket: Arc<NetSocket>, state: Shared, mut game: Game) {
    let mut tick: u32 = 0;
    let mut next_tick = TokioInstant::now();

    loop {
        sleep_until(next_tick).await;

        // Apply latest input for every live session.
        let (game_started, addrs): (bool, Vec<SocketAddr>) = {
            let mut st = state.lock().unwrap();
            for session in st.sessions.values_mut() {
                if session.health == 0 {
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

        // Combat is only stepped once the match has started.
        let bullets = if game_started {
            let mut st = state.lock().unwrap();
            step_combat(&mut st, &mut game.walls, TICK_DURATION.as_secs_f32())
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
