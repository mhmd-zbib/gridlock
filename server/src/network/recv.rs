use std::net::SocketAddr;
use std::sync::Arc;

use crate::session::{ServerState, Session, Shared};
use crate::util::{default_weapon_state, server_time_ms, spawn_offset};
use net::proto::server::MovementState;
use net::{
    AnyPacket, ConnectAck, ConnectResult, LobbyCommandKind, LobbyPlayer, LobbyState, NetSocket,
    PROTOCOL_VERSION, encode, encode_rotation,
};

const MAX_PLAYERS: usize = 16;
const MAX_HEALTH: u16 = 100;

// ---------------------------------------------------------------------------
// Receive loop
// ---------------------------------------------------------------------------

pub async fn recv_loop(socket: Arc<NetSocket>, state: Shared) {
    loop {
        match socket.recv().await {
            Ok((packet, addr)) => handle_packet(packet, addr, &socket, &state).await,
            Err(_) => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Packet handlers
// ---------------------------------------------------------------------------

async fn handle_packet(packet: AnyPacket, addr: SocketAddr, socket: &NetSocket, state: &Shared) {
    match packet {
        AnyPacket::ConnectRequest(req) => handle_connect(req, addr, socket, state).await,
        AnyPacket::Disconnect => handle_disconnect(addr, socket, state).await,
        AnyPacket::ClientInput(input) => {
            if let Some(session) = state.lock().unwrap().sessions.get_mut(&addr) {
                // Spectators don't send movement inputs to the server simulation.
                if !session.is_spectator {
                    session.latest_input = Some(input);
                }
            }
        }
        AnyPacket::LobbyCommand(cmd) => handle_lobby_command(cmd, addr, socket, state).await,
        AnyPacket::Ping(ts) => {
            let _ = socket.send_to(&AnyPacket::Pong(ts), addr).await;
        }
        _ => {}
    }
}

async fn handle_connect(
    req: net::ConnectRequest,
    addr: SocketAddr,
    socket: &NetSocket,
    state: &Shared,
) {
    enum Decision {
        ReAck(u16),
        NewPlayer(u16),
        Full,
        VersionMismatch,
    }

    let decision = {
        let mut st = state.lock().unwrap();
        if let Some(s) = st.sessions.get(&addr) {
            Decision::ReAck(s.player_id)
        } else if req.version != PROTOCOL_VERSION {
            Decision::VersionMismatch
        } else if st.sessions.len() >= MAX_PLAYERS {
            Decision::Full
        } else {
            let pid = st.next_player_id;
            st.next_player_id = pid.wrapping_add(1).max(1);
            let spawn = st.spawn;
            let offset = spawn_offset(pid);
            let weapon = default_weapon_state();

            // Mid-game joiners start as spectators with no health so the round
            // system doesn't count them and combat ignores them.
            let is_spectator = st.game_started;
            let initial_health = if is_spectator { 0 } else { MAX_HEALTH };

            st.sessions.insert(
                addr,
                Session {
                    player_id: pid,
                    name: req.name_str().to_string(),
                    x: spawn.0 + offset.0,
                    y: spawn.1 + offset.1,
                    health: initial_health,
                    rotation: encode_rotation(0.0),
                    weapon,
                    aim_cone: game::world::aim_cone::AimCone::new(),
                    movement_state: MovementState::default(),
                    team: 0,
                    is_spectator,
                    latest_input: None,
                },
            );

            // First player to connect becomes the room creator.
            if st.creator_addr.is_none() {
                st.creator_addr = Some(addr);
            }

            Decision::NewPlayer(pid)
        }
    };

    let tick_rate = 60u8;
    let time = server_time_ms();

    match decision {
        Decision::Full => {
            let _ = socket
                .send_to(
                    &AnyPacket::ConnectAck(ConnectAck {
                        result: ConnectResult::ServerFull,
                        player_id: 0,
                        tick_rate,
                        server_time: time,
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
                        tick_rate,
                        server_time: time,
                    }),
                    addr,
                )
                .await;
        }
        Decision::ReAck(pid) | Decision::NewPlayer(pid) => {
            let _ = socket.accept(&req, addr, pid, tick_rate, time).await;
            broadcast_lobby_state(socket, state).await;
        }
    }
}

async fn handle_disconnect(addr: SocketAddr, socket: &NetSocket, state: &Shared) {
    {
        let mut st = state.lock().unwrap();
        st.sessions.remove(&addr);
        // Transfer creator role to the next connected player when the creator
        // leaves before the game has started.
        if st.creator_addr == Some(addr) && !st.game_started {
            st.creator_addr = st.sessions.keys().next().copied();
        }
    }
    broadcast_lobby_state(socket, state).await;
}

async fn handle_lobby_command(
    cmd: net::LobbyCommand,
    addr: SocketAddr,
    socket: &NetSocket,
    state: &Shared,
) {
    let mut should_broadcast = false;
    {
        let mut st = state.lock().unwrap();

        match cmd.kind {
            LobbyCommandKind::SelectTeam => {
                let game_started = st.game_started;
                if let Some(session) = st.sessions.get_mut(&addr) {
                    // Block team switching once the player has locked in mid-game.
                    if !game_started || session.team == 0 {
                        session.team = if cmd.team == 2 { 2 } else { 1 };
                        should_broadcast = true;
                    }
                }
            }
            LobbyCommandKind::StartGame => {
                // Only the room creator can start, and only before the game begins.
                let is_creator = st.creator_addr == Some(addr);
                if is_creator && !st.game_started && st.sessions.contains_key(&addr) {
                    st.game_started = true;
                    should_broadcast = true;
                    let team1_spawn = st.team1_spawn;
                    let team2_spawn = st.team2_spawn;
                    let default_spawn = st.spawn;
                    for session in st.sessions.values_mut() {
                        // Players without a team stay as spectators.
                        if session.team == 0 {
                            session.is_spectator = true;
                            session.health = 0;
                            continue;
                        }
                        let base = match session.team {
                            1 => team1_spawn.unwrap_or(default_spawn),
                            2 => team2_spawn.unwrap_or(default_spawn),
                            _ => default_spawn,
                        };
                        let offset = spawn_offset(session.player_id);
                        session.x = base.0 + offset.0;
                        session.y = base.1 + offset.1;
                        session.health = MAX_HEALTH;
                        session.is_spectator = false;
                    }
                }
            }
        }
    }
    if should_broadcast {
        broadcast_lobby_state(socket, state).await;
    }
}

// ---------------------------------------------------------------------------
// Lobby broadcast
// ---------------------------------------------------------------------------

pub async fn broadcast_lobby_state(socket: &NetSocket, state: &Shared) {
    let per_client: Vec<(SocketAddr, Vec<u8>)> = {
        let st = state.lock().unwrap();
        st.sessions
            .keys()
            .copied()
            .map(|addr| {
                let lobby = lobby_state_for(&st, addr);
                let payload = encode(&AnyPacket::LobbyState(lobby));
                (addr, payload)
            })
            .collect()
    };

    for (addr, payload) in per_client {
        let _ = socket.send_raw(&payload, addr).await;
    }
}

fn lobby_state_for(st: &ServerState, addr: SocketAddr) -> LobbyState {
    let mut team1_count = 0u8;
    let mut team2_count = 0u8;

    // Build the roster visible to all clients (all players, sorted by team).
    let mut players: Vec<LobbyPlayer> = st
        .sessions
        .values()
        .map(|s| {
            if s.team == 1 {
                team1_count = team1_count.saturating_add(1);
            } else if s.team == 2 {
                team2_count = team2_count.saturating_add(1);
            }
            let mut name = [0u8; 16];
            let bytes = s.name.as_bytes();
            let len = bytes.len().min(16);
            name[..len].copy_from_slice(&bytes[..len]);
            LobbyPlayer { name, team: s.team }
        })
        .collect();

    // Stable ordering: team 1, then team 2, then unassigned.
    players.sort_by_key(|p| match p.team {
        1 => 0u8,
        2 => 1,
        _ => 2,
    });

    LobbyState {
        game_started: st.game_started,
        your_team: st.sessions.get(&addr).map(|s| s.team).unwrap_or(0),
        team1_count,
        team2_count,
        is_creator: st.creator_addr == Some(addr),
        players,
    }
}
