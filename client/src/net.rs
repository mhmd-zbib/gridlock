use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use net::{AnyPacket, ClientPacket, ConnectResult, NetSocket};
use net::proto::server::ServerPacket;

// ── Public types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum ConnState {
    Connecting,
    Connected { player_id: u16 },
    Rejected(String),
    Disconnected,
}

enum Cmd {
    Disconnect,
    SendInput(ClientPacket),
}

// ── NetClient ─────────────────────────────────────────────────────────────────

pub struct NetClient {
    state: Arc<Mutex<ConnState>>,
    cmd_tx: std::sync::mpsc::SyncSender<Cmd>,
    /// Latest snapshot received from the server; replaced each tick.
    latest_snapshot: Arc<Mutex<Option<ServerPacket>>>,
}

impl NetClient {
    pub fn connect(server_addr: SocketAddr, name: String) -> Self {
        let state   = Arc::new(Mutex::new(ConnState::Connecting));
        let state_bg = Arc::clone(&state);
        let (cmd_tx, cmd_rx) = std::sync::mpsc::sync_channel(8);
        let latest_snapshot = Arc::new(Mutex::new(None::<ServerPacket>));
        let snapshot_bg = Arc::clone(&latest_snapshot);

        std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(net_task(server_addr, name, state_bg, cmd_rx, snapshot_bg));
        });

        Self { state, cmd_tx, latest_snapshot }
    }

    pub fn state(&self) -> ConnState {
        self.state.lock().unwrap().clone()
    }

    pub fn disconnect(&self) {
        let _ = self.cmd_tx.try_send(Cmd::Disconnect);
    }

    pub fn send_input(&self, pkt: ClientPacket) {
        let _ = self.cmd_tx.try_send(Cmd::SendInput(pkt));
    }

    /// Take the most recent server snapshot, leaving `None` behind.
    ///
    /// Returns `None` if no new snapshot has arrived since the last call.
    pub fn take_snapshot(&self) -> Option<ServerPacket> {
        self.latest_snapshot.lock().unwrap().take()
    }
}

impl Drop for NetClient {
    fn drop(&mut self) {
        self.disconnect();
    }
}

// ── Background task ───────────────────────────────────────────────────────────

async fn net_task(
    server_addr: SocketAddr,
    name: String,
    state: Arc<Mutex<ConnState>>,
    cmd_rx: std::sync::mpsc::Receiver<Cmd>,
    latest_snapshot: Arc<Mutex<Option<ServerPacket>>>,
) {
    let socket = match NetSocket::bind("0.0.0.0:0".parse().unwrap()).await {
        Ok(s) => s,
        Err(e) => {
            *state.lock().unwrap() = ConnState::Rejected(format!("socket error: {e}"));
            return;
        }
    };

    // Handshake — 5 s timeout
    let ack = match tokio::time::timeout(
        Duration::from_secs(5),
        socket.connect(server_addr, &name),
    )
    .await
    {
        Ok(Ok(a))  => a,
        Ok(Err(e)) => { *state.lock().unwrap() = ConnState::Rejected(e.to_string()); return; }
        Err(_)     => { *state.lock().unwrap() = ConnState::Rejected("timed out".into()); return; }
    };

    match ack.result {
        ConnectResult::Ok => {
            *state.lock().unwrap() = ConnState::Connected { player_id: ack.player_id };
        }
        ConnectResult::ServerFull     => { *state.lock().unwrap() = ConnState::Rejected("server full".into());             return; }
        ConnectResult::VersionMismatch => { *state.lock().unwrap() = ConnState::Rejected("version mismatch".into());       return; }
        ConnectResult::Banned         => { *state.lock().unwrap() = ConnState::Rejected("banned".into());                  return; }
    }

    // Session loop — stay connected until explicit Disconnect or server closes
    loop {
        // Drain all pending commands
        loop {
            match cmd_rx.try_recv() {
                Ok(Cmd::Disconnect) => {
                    let _ = socket.send_to(&AnyPacket::Disconnect, server_addr).await;
                    *state.lock().unwrap() = ConnState::Disconnected;
                    return;
                }
                Ok(Cmd::SendInput(pkt)) => {
                    let _ = socket.send_to(&AnyPacket::ClientInput(pkt), server_addr).await;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    let _ = socket.send_to(&AnyPacket::Disconnect, server_addr).await;
                    return;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
            }
        }

        // Non-blocking recv — store snapshots, react to disconnect
        match tokio::time::timeout(Duration::from_millis(16), socket.recv()).await {
            Ok(Ok((AnyPacket::ServerSnapshot(snap), _))) => {
                println!(
                    "[net] snapshot tick={} bullets={} players={} sounds={}",
                    snap.tick,
                    snap.bullets.len(),
                    snap.players.len(),
                    snap.sounds.len(),
                );
                for b in &snap.bullets {
                    println!(
                        "[net]   bullet shooter={} ({:.2},{:.2}) -> ({:.2},{:.2}) hit={}",
                        b.shooter_id, b.from_x, b.from_y, b.to_x, b.to_y, b.hit_player_id,
                    );
                }
                *latest_snapshot.lock().unwrap() = Some(snap);
            }
            Ok(Ok((AnyPacket::Disconnect, _))) => {
                *state.lock().unwrap() = ConnState::Disconnected;
                return;
            }
            _ => {}
        }
    }
}
