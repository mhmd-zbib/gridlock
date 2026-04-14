pub mod codec;
pub mod proto;
pub mod socket;

// ── Flat re-exports for convenience ──────────────────────────────────────────

pub use codec::{decode, decode_rotation, encode, encode_rotation, DecodeError};
pub use proto::{
    client::{ClientPacket, InputFlags},
    handshake::{
        AnyPacket, ConnectAck, ConnectRequest, ConnectResult, PacketKind, PROTOCOL_VERSION,
    },
    server::{
        BulletEvent, MatchState, MovementState, PlayerState, SelfState, ServerPacket, SoundEvent,
        SoundKind,
    },
};
pub use socket::{NetSocket, RecvError, MAX_DATAGRAM};
