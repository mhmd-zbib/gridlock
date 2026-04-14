use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use net::proto::server::ServerPacket;
use net::{AnyPacket, ClientPacket, ConnectResult, NetSocket};

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

/// Maximum number of snapshots buffered before older ones are dropped.
const MAX_PENDING_SNAPSHOTS: usize = 16;

pub struct NetClient {
    state: Arc<Mutex<ConnState>>,
    cmd_tx: std::sync::mpsc::SyncSender<Cmd>,
    /// All snapshots received from the server since the last `take_snapshots()`
    /// call, in arrival order.  Using a queue instead of a single slot prevents
    /// bullet events from being silently discarded when the network thread
    /// receives multiple snapshots between client game ticks.
    pending_snapshots: Arc<Mutex<VecDeque<ServerPacket>>>,
}

impl NetClient {
    pub fn connect(server_addr: SocketAddr, name: String) -> Self {
        let state = Arc::new(Mutex::new(ConnState::Connecting));
        let state_bg = Arc::clone(&state);
        let (cmd_tx, cmd_rx) = std::sync::mpsc::sync_channel(8);
        let pending_snapshots = Arc::new(Mutex::new(VecDeque::new()));
        let snapshot_bg = Arc::clone(&pending_snapshots);

        std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(net_task(server_addr, name, state_bg, cmd_rx, snapshot_bg));
        });

        Self {
            state,
            cmd_tx,
            pending_snapshots,
        }
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

    /// Drain all server snapshots received since the last call, oldest first.
    ///
    /// Returns an empty `Vec` if no new snapshots have arrived.
    /// Callers should use the last entry's state fields (health, ammo, …) as
    /// the authoritative current state, and accumulate bullet/sound events from
    /// **all** entries so that no projectile events are dropped between ticks.
    pub fn take_snapshots(&self) -> Vec<ServerPacket> {
        self.pending_snapshots.lock().unwrap().drain(..).collect()
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
    pending_snapshots: Arc<Mutex<VecDeque<ServerPacket>>>,
) {
    let socket = match NetSocket::bind("0.0.0.0:0".parse().unwrap()).await {
        Ok(s) => s,
        Err(e) => {
            *state.lock().unwrap() = ConnState::Rejected(format!("socket error: {e}"));
            return;
        }
    };

    // Handshake — 5 s timeout
    let ack = match tokio::time::timeout(Duration::from_secs(5), socket.connect(server_addr, &name))
        .await
    {
        Ok(Ok(a)) => a,
        Ok(Err(e)) => {
            *state.lock().unwrap() = ConnState::Rejected(e.to_string());
            return;
        }
        Err(_) => {
            *state.lock().unwrap() = ConnState::Rejected("timed out".into());
            return;
        }
    };

    match ack.result {
        ConnectResult::Ok => {
            *state.lock().unwrap() = ConnState::Connected {
                player_id: ack.player_id,
            };
        }
        ConnectResult::ServerFull => {
            *state.lock().unwrap() = ConnState::Rejected("server full".into());
            return;
        }
        ConnectResult::VersionMismatch => {
            *state.lock().unwrap() = ConnState::Rejected("version mismatch".into());
            return;
        }
        ConnectResult::Banned => {
            *state.lock().unwrap() = ConnState::Rejected("banned".into());
            return;
        }
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
                    let _ = socket
                        .send_to(&AnyPacket::ClientInput(pkt), server_addr)
                        .await;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    let _ = socket.send_to(&AnyPacket::Disconnect, server_addr).await;
                    return;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
            }
        }

        // Non-blocking recv — queue snapshots, react to disconnect
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
                let mut queue = pending_snapshots.lock().unwrap();
                if queue.len() >= MAX_PENDING_SNAPSHOTS {
                    queue.pop_front(); // drop oldest to bound memory
                }
                queue.push_back(snap);
            }
            Ok(Ok((AnyPacket::Disconnect, _))) => {
                *state.lock().unwrap() = ConnState::Disconnected;
                return;
            }
            _ => {}
        }
    }
}
