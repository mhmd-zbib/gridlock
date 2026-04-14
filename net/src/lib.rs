pub mod codec;
pub mod proto;
pub mod socket;

// ── Flat re-exports for convenience ──────────────────────────────────────────

pub use codec::{DecodeError, decode, decode_rotation, encode, encode_rotation};
pub use proto::MoveSpeed;
pub use proto::{
    client::{ClientPacket, InputFlags},
    handshake::{
        AnyPacket, ConnectAck, ConnectRequest, ConnectResult, PROTOCOL_VERSION, PacketKind,
    },
    lobby::{LobbyCommand, LobbyCommandKind, LobbyState},
    server::{
        BulletEvent, MatchState, MovementState, PlayerState, SelfState, ServerPacket, SoundEvent,
        SoundKind,
    },
};
pub use socket::{MAX_DATAGRAM, NetSocket, RecvError};
