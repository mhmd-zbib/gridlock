/// First byte of every UDP datagram — tells the receiver which packet follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PacketKind {
    /// Client → server: initiate a session.
    ConnectRequest = 0x01,
    /// Server → client: session accepted or rejected.
    ConnectAck = 0x02,
    /// Client → server: per-tick input state.
    ClientInput = 0x03,
    /// Server → client: per-tick authoritative snapshot.
    ServerSnapshot = 0x04,
    /// Either direction: orderly teardown.
    Disconnect = 0x05,
    /// Either direction: heartbeat / latency probe (no payload).
    Ping = 0x06,
    /// Either direction: response to a Ping.
    Pong = 0x07,
}

impl PacketKind {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0x01 => Some(Self::ConnectRequest),
            0x02 => Some(Self::ConnectAck),
            0x03 => Some(Self::ClientInput),
            0x04 => Some(Self::ServerSnapshot),
            0x05 => Some(Self::Disconnect),
            0x06 => Some(Self::Ping),
            0x07 => Some(Self::Pong),
            _ => None,
        }
    }
}

// ── ConnectRequest ────────────────────────────────────────────────────────────

/// Sent by the client to open a session (18 bytes on the wire).
///
/// Wire layout: [version: u8] [name: [u8; 16]]
#[derive(Debug, Clone, Copy)]
pub struct ConnectRequest {
    /// Protocol version; must match [`PROTOCOL_VERSION`].
    pub version: u8,

    /// Player display name as UTF-8 bytes, right-padded with `0x00`.
    /// Names longer than 16 bytes are silently truncated.
    pub name: [u8; 16],
}

impl ConnectRequest {
    /// Creates a request with `name` truncated / zero-padded to 16 bytes.
    pub fn new(name: &str) -> Self {
        let mut buf = [0u8; 16];
        let bytes = name.as_bytes();
        let len = bytes.len().min(16);
        buf[..len].copy_from_slice(&bytes[..len]);
        Self {
            version: PROTOCOL_VERSION,
            name: buf,
        }
    }

    /// Returns the player name with trailing `NUL` bytes stripped.
    pub fn name_str(&self) -> &str {
        let end = self.name.iter().position(|&b| b == 0).unwrap_or(16);
        std::str::from_utf8(&self.name[..end]).unwrap_or("")
    }
}

// ── ConnectAck ────────────────────────────────────────────────────────────────

/// Sent by the server in response to a [`ConnectRequest`] (9 bytes on the wire).
///
/// Wire layout: [result: u8] [player_id: u16] [tick_rate: u8] [server_time: u32]
#[derive(Debug, Clone, Copy)]
pub struct ConnectAck {
    /// Whether the session was accepted; see [`ConnectResult`].
    pub result: ConnectResult,

    /// Server-assigned player identifier (only meaningful when `result == Ok`).
    pub player_id: u16,

    /// Server tick rate in Hz (e.g. `60`).
    pub tick_rate: u8,

    /// Server clock in milliseconds at the moment of sending — used by the
    /// client to bootstrap clock synchronisation.
    pub server_time: u32,
}

/// Outcome of a [`ConnectRequest`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ConnectResult {
    Ok = 0,
    ServerFull = 1,
    VersionMismatch = 2,
    Banned = 3,
}

impl ConnectResult {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Ok),
            1 => Some(Self::ServerFull),
            2 => Some(Self::VersionMismatch),
            3 => Some(Self::Banned),
            _ => None,
        }
    }
}

// ── AnyPacket ─────────────────────────────────────────────────────────────────

use crate::proto::{client::ClientPacket, server::ServerPacket};

/// Tagged union of every packet type — the surface returned by
/// [`crate::codec::decode`] and accepted by [`crate::codec::encode`].
#[derive(Debug, Clone)]
pub enum AnyPacket {
    ConnectRequest(ConnectRequest),
    ConnectAck(ConnectAck),
    ClientInput(ClientPacket),
    ServerSnapshot(ServerPacket),
    Disconnect,
    Ping(u32),
    Pong(u32),
}

impl AnyPacket {
    pub fn kind(&self) -> PacketKind {
        match self {
            Self::ConnectRequest(_) => PacketKind::ConnectRequest,
            Self::ConnectAck(_) => PacketKind::ConnectAck,
            Self::ClientInput(_) => PacketKind::ClientInput,
            Self::ServerSnapshot(_) => PacketKind::ServerSnapshot,
            Self::Disconnect => PacketKind::Disconnect,
            Self::Ping(_) => PacketKind::Ping,
            Self::Pong(_) => PacketKind::Pong,
        }
    }
}

// ── Constants ─────────────────────────────────────────────────────────────────

/// Bump this whenever the wire format changes in a backwards-incompatible way.
pub const PROTOCOL_VERSION: u8 = 1;
