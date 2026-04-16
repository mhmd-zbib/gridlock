use std::net::SocketAddr;
use std::sync::Arc;

use crate::session::{ServerState, Session, Shared};
use crate::util::{default_weapon_state, server_time_ms, spawn_offset};
use net::proto::server::MovementState;
use net::{
    AnyPacket, ConnectAck, ConnectResult, LobbyCommandKind, LobbyState, NetSocket,
    PROTOCOL_VERSION, encode, encode_rotation,
};

const MAX_PLAYERS: usize = 16;
const MAX_HEALTH: u16 = 100;

// ---------------------------------------------------------------------------
// Receive loop
// ---------------------------------------------------------------------------

/// Continuously receive packets from the UDP socket and dispatch them.
///
/// This runs as a dedicated async task so it never blocks the tick loop.
pub async fn recv_loop(socket: Arc<NetSocket>, state: Shared) {
    loop {
        match socket.recv().await {
            Ok((packet, addr)) => handle_packet(packet, addr, &socket, &state).await,
            Err(_) => {} // decode errors and oversized packets are silently dropped
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
                session.latest_input = Some(input);
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
            let offset = spawn_offset(pid);
            let weapon = default_weapon_state();
            let aim_cone_half_angle = weapon.stats().aim_base_half_angle_deg.to_radians();
            st.sessions.insert(
                addr,
                Session {
                    player_id: pid,
                    name: req.name_str().to_string(),
                    x: spawn.0 + offset.0,
                    y: spawn.1 + offset.1,
                    health: MAX_HEALTH,
                    rotation: encode_rotation(0.0),
                    weapon,
                    aim_cone_half_angle,
                    movement_state: MovementState::default(),
                    team: 0,
                    latest_input: None,
                },
            );
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
        Decision::MatchStarted => {
            let _ = socket
                .send_to(
                    &AnyPacket::ConnectAck(ConnectAck {
                        result: ConnectResult::MatchStarted,
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
    state.lock().unwrap().sessions.remove(&addr);
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

// ---------------------------------------------------------------------------
// Lobby broadcast
// ---------------------------------------------------------------------------

/// Send the current `LobbyState` to every connected client.
///
/// Each client receives a personalised view (`your_team` reflects its own team
/// selection) so this always broadcasts individually rather than as a multicast.
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
