//! Binary encode / decode for all wire packets.
//!
//! Every datagram starts with a [`PacketKind`] discriminant byte, followed by
//! the packet body in **big-endian** network byte order.
//!
//! # Entry points
//! - [`encode`] — serialise an [`AnyPacket`] into a fresh `Vec<u8>`.
//! - [`decode`] — parse a raw datagram buffer into an [`AnyPacket`].
//! - [`encode_rotation`] / [`decode_rotation`] — helpers for the `u16` ↔ `f32`
//!   angle format used in `ClientPacket::rotation` and `PlayerState::rotation`.

use crate::proto::{
    client::{ClientPacket, InputFlags},
    handshake::{AnyPacket, ConnectAck, ConnectRequest, ConnectResult, PacketKind},
    server::{
        BulletEvent, MatchState, MovementState, PlayerState, SelfState, ServerPacket, SoundEvent,
        SoundKind,
    },
};

// ── Public entry points ───────────────────────────────────────────────────────

/// Serialise `packet` into a newly allocated byte vector ready to hand to the
/// socket layer.  The first byte is always the [`PacketKind`] discriminant.
pub fn encode(packet: &AnyPacket) -> Vec<u8> {
    let mut w = BufWriter::new();
    match packet {
        AnyPacket::ConnectRequest(p) => {
            w.u8(PacketKind::ConnectRequest as u8);
            w.u8(p.version);
            w.bytes(&p.name);
        }
        AnyPacket::ConnectAck(p) => {
            w.u8(PacketKind::ConnectAck as u8);
            w.u8(p.result as u8);
            w.u16(p.player_id);
            w.u8(p.tick_rate);
            w.u32(p.server_time);
        }
        AnyPacket::ClientInput(p) => {
            w.u8(PacketKind::ClientInput as u8);
            w.u16(p.sequence);
            w.u32(p.timestamp);
            w.i8(p.movement_x);
            w.i8(p.movement_y);
            w.u16(p.rotation);
            w.u8(p.flags.0);
        }
        AnyPacket::ServerSnapshot(p) => {
            w.u8(PacketKind::ServerSnapshot as u8);
            w.u32(p.tick);
            w.u32(p.timestamp);
            // self state
            w.u8(p.me.health);
            w.u8(p.me.ammo);
            w.u8(p.me.reload_progress);
            w.u8(p.me.movement_state.0);
            // players
            w.u8(p.players.len().min(255) as u8);
            for pl in p.players.iter().take(255) {
                w.u16(pl.id);
                w.f32(pl.x);
                w.f32(pl.y);
                w.u16(pl.rotation);
                w.u8(pl.movement_state.0);
                w.u8(pl.weapon);
            }
            // bullets
            w.u8(p.bullets.len().min(255) as u8);
            for b in p.bullets.iter().take(255) {
                w.u16(b.shooter_id);
                w.f32(b.from_x);
                w.f32(b.from_y);
                w.f32(b.to_x);
                w.f32(b.to_y);
                w.u16(b.hit_player_id);
            }
            // sounds
            w.u8(p.sounds.len().min(255) as u8);
            for s in p.sounds.iter().take(255) {
                w.u8(s.kind as u8);
                w.f32(s.x);
                w.f32(s.y);
                w.u8(s.intensity);
            }
            // match state
            w.u16(p.match_state.timer);
            w.u8(p.match_state.score_team1);
            w.u8(p.match_state.score_team2);
        }
        AnyPacket::Disconnect => {
            w.u8(PacketKind::Disconnect as u8);
        }
        AnyPacket::Ping(ts) => {
            w.u8(PacketKind::Ping as u8);
            w.u32(*ts);
        }
        AnyPacket::Pong(ts) => {
            w.u8(PacketKind::Pong as u8);
            w.u32(*ts);
        }
    }
    w.finish()
}

/// Parse a raw datagram buffer into an [`AnyPacket`].
///
/// Returns [`DecodeError::UnknownKind`] for unrecognised discriminants, and
/// [`DecodeError::UnexpectedEof`] when the buffer is shorter than expected.
pub fn decode(buf: &[u8]) -> Result<AnyPacket, DecodeError> {
    let mut r = BufReader::new(buf);
    let kind_byte = r.u8()?;
    let kind = PacketKind::from_u8(kind_byte).ok_or(DecodeError::UnknownKind(kind_byte))?;

    let packet = match kind {
        PacketKind::ConnectRequest => {
            let version = r.u8()?;
            let mut name = [0u8; 16];
            name.copy_from_slice(r.bytes(16)?);
            AnyPacket::ConnectRequest(ConnectRequest { version, name })
        }
        PacketKind::ConnectAck => {
            let result_byte = r.u8()?;
            let result =
                ConnectResult::from_u8(result_byte).ok_or(DecodeError::InvalidField("result"))?;
            let player_id = r.u16()?;
            let tick_rate = r.u8()?;
            let server_time = r.u32()?;
            AnyPacket::ConnectAck(ConnectAck {
                result,
                player_id,
                tick_rate,
                server_time,
            })
        }
        PacketKind::ClientInput => {
            let sequence = r.u16()?;
            let timestamp = r.u32()?;
            let movement_x = r.i8()?;
            let movement_y = r.i8()?;
            let rotation = r.u16()?;
            let flags = InputFlags(r.u8()?);
            AnyPacket::ClientInput(ClientPacket {
                sequence,
                timestamp,
                movement_x,
                movement_y,
                rotation,
                flags,
            })
        }
        PacketKind::ServerSnapshot => {
            let tick = r.u32()?;
            let timestamp = r.u32()?;
            let me = SelfState {
                health: r.u8()?,
                ammo: r.u8()?,
                reload_progress: r.u8()?,
                movement_state: MovementState(r.u8()?),
            };
            let player_count = r.u8()? as usize;
            let mut players = Vec::with_capacity(player_count);
            for _ in 0..player_count {
                players.push(PlayerState {
                    id: r.u16()?,
                    x: r.f32()?,
                    y: r.f32()?,
                    rotation: r.u16()?,
                    movement_state: MovementState(r.u8()?),
                    weapon: r.u8()?,
                });
            }
            let bullet_count = r.u8()? as usize;
            let mut bullets = Vec::with_capacity(bullet_count);
            for _ in 0..bullet_count {
                bullets.push(BulletEvent {
                    shooter_id: r.u16()?,
                    from_x: r.f32()?,
                    from_y: r.f32()?,
                    to_x: r.f32()?,
                    to_y: r.f32()?,
                    hit_player_id: r.u16()?,
                });
            }
            let sound_count = r.u8()? as usize;
            let mut sounds = Vec::with_capacity(sound_count);
            for _ in 0..sound_count {
                let kind_byte = r.u8()?;
                let kind = SoundKind::from_u8(kind_byte)
                    .ok_or(DecodeError::InvalidField("sound_kind"))?;
                sounds.push(SoundEvent {
                    kind,
                    x: r.f32()?,
                    y: r.f32()?,
                    intensity: r.u8()?,
                });
            }
            let match_state = MatchState {
                timer: r.u16()?,
                score_team1: r.u8()?,
                score_team2: r.u8()?,
            };
            AnyPacket::ServerSnapshot(ServerPacket {
                tick,
                timestamp,
                me,
                players,
                bullets,
                sounds,
                match_state,
            })
        }
        PacketKind::Disconnect => AnyPacket::Disconnect,
        PacketKind::Ping => AnyPacket::Ping(r.u32()?),
        PacketKind::Pong => AnyPacket::Pong(r.u32()?),
    };

    Ok(packet)
}

// ── Rotation helpers ──────────────────────────────────────────────────────────

/// Compress a radian angle to a `u16` for wire transmission.
///
/// Any input is normalised to `[0, 2π)` before encoding, so values outside
/// that range are safe to pass.
#[inline]
pub fn encode_rotation(rad: f32) -> u16 {
    let normalised = rad.rem_euclid(std::f32::consts::TAU);
    ((normalised / std::f32::consts::TAU) * 65535.0) as u16
}

/// Expand a wire-format rotation back to radians in `[0, 2π)`.
#[inline]
pub fn decode_rotation(v: u16) -> f32 {
    (v as f32 / 65535.0) * std::f32::consts::TAU
}

// ── DecodeError ───────────────────────────────────────────────────────────────

/// Errors that can occur while decoding a datagram.
#[derive(Debug)]
pub enum DecodeError {
    /// The buffer ended before all expected bytes were read.
    UnexpectedEof,
    /// The first byte does not correspond to any known [`PacketKind`].
    UnknownKind(u8),
    /// A field value was out of range or otherwise invalid.
    InvalidField(&'static str),
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnexpectedEof => write!(f, "unexpected end of datagram"),
            Self::UnknownKind(k) => write!(f, "unknown packet kind: 0x{k:02x}"),
            Self::InvalidField(name) => write!(f, "invalid value in field `{name}`"),
        }
    }
}

impl std::error::Error for DecodeError {}

// ── BufWriter (private) ───────────────────────────────────────────────────────

struct BufWriter(Vec<u8>);

impl BufWriter {
    fn new() -> Self {
        Self(Vec::with_capacity(64))
    }
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn i8(&mut self, v: i8) {
        self.0.push(v as u8);
    }
    fn u16(&mut self, v: u16) {
        self.0.extend_from_slice(&v.to_be_bytes());
    }
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_be_bytes());
    }
    fn f32(&mut self, v: f32) {
        self.0.extend_from_slice(&v.to_bits().to_be_bytes());
    }
    fn bytes(&mut self, v: &[u8]) {
        self.0.extend_from_slice(v);
    }
    fn finish(self) -> Vec<u8> {
        self.0
    }
}

// ── BufReader (private) ───────────────────────────────────────────────────────

struct BufReader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> BufReader<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    fn u8(&mut self) -> Result<u8, DecodeError> {
        self.buf
            .get(self.pos)
            .map(|&b| {
                self.pos += 1;
                b
            })
            .ok_or(DecodeError::UnexpectedEof)
    }

    fn i8(&mut self) -> Result<i8, DecodeError> {
        self.u8().map(|b| b as i8)
    }

    fn u16(&mut self) -> Result<u16, DecodeError> {
        self.bytes(2)
            .map(|b| u16::from_be_bytes(b.try_into().unwrap()))
    }

    fn u32(&mut self) -> Result<u32, DecodeError> {
        self.bytes(4)
            .map(|b| u32::from_be_bytes(b.try_into().unwrap()))
    }

    fn f32(&mut self) -> Result<f32, DecodeError> {
        self.bytes(4)
            .map(|b| f32::from_bits(u32::from_be_bytes(b.try_into().unwrap())))
    }

    fn bytes(&mut self, n: usize) -> Result<&'a [u8], DecodeError> {
        let end = self.pos + n;
        if end > self.buf.len() {
            return Err(DecodeError::UnexpectedEof);
        }
        let slice = &self.buf[self.pos..end];
        self.pos = end;
        Ok(slice)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::{
        client::InputFlags,
        handshake::{ConnectRequest, PROTOCOL_VERSION},
        server::SoundKind,
    };

    #[test]
    fn roundtrip_connect_request() {
        let original = AnyPacket::ConnectRequest(ConnectRequest::new("TestPlayer"));
        let buf = encode(&original);
        let decoded = decode(&buf).unwrap();
        let AnyPacket::ConnectRequest(p) = decoded else {
            panic!("wrong variant");
        };
        assert_eq!(p.version, PROTOCOL_VERSION);
        assert_eq!(p.name_str(), "TestPlayer");
    }

    #[test]
    fn roundtrip_client_input() {
        let mut flags = InputFlags::default();
        flags.set_shooting(true);
        flags.set_weapon_slot(3);
        let original = AnyPacket::ClientInput(ClientPacket {
            sequence: 1234,
            timestamp: 9_000_000,
            movement_x: -1,
            movement_y: 1,
            rotation: encode_rotation(1.5707964), // ~π/2
            flags,
        });
        let buf = encode(&original);
        let decoded = decode(&buf).unwrap();
        let AnyPacket::ClientInput(p) = decoded else {
            panic!("wrong variant");
        };
        assert_eq!(p.sequence, 1234);
        assert_eq!(p.movement_x, -1);
        assert!(p.flags.is_shooting());
        assert_eq!(p.flags.weapon_slot(), 3);
    }

    #[test]
    fn roundtrip_server_snapshot() {
        let snap = ServerPacket {
            tick: 42,
            timestamp: 1000,
            me: SelfState {
                health: 100,
                ammo: 30,
                reload_progress: 0,
                movement_state: MovementState(0),
            },
            players: vec![PlayerState {
                id: 7,
                x: 3.5,
                y: 12.25,
                rotation: encode_rotation(0.0),
                movement_state: MovementState(0),
                weapon: 1,
            }],
            bullets: vec![BulletEvent {
                shooter_id: 7,
                from_x: 3.5,
                from_y: 12.25,
                to_x: 10.0,
                to_y: 15.0,
                hit_player_id: 0,
            }],
            sounds: vec![SoundEvent {
                kind: SoundKind::Gunshot,
                x: 3.5,
                y: 12.25,
                intensity: 200,
            }],
            match_state: MatchState {
                timer: 180,
                score_team1: 3,
                score_team2: 1,
            },
        };
        let buf = encode(&AnyPacket::ServerSnapshot(snap));
        let decoded = decode(&buf).unwrap();
        let AnyPacket::ServerSnapshot(p) = decoded else {
            panic!("wrong variant");
        };
        assert_eq!(p.tick, 42);
        assert_eq!(p.me.health, 100);
        assert_eq!(p.players.len(), 1);
        assert_eq!(p.players[0].id, 7);
        assert!((p.players[0].x - 3.5).abs() < 1e-5);
        assert_eq!(p.bullets.len(), 1);
        assert_eq!(p.sounds.len(), 1);
        assert_eq!(p.sounds[0].kind, SoundKind::Gunshot);
        assert_eq!(p.match_state.timer, 180);
    }

    #[test]
    fn rotation_roundtrip() {
        for deg in [0.0f32, 45.0, 90.0, 180.0, 270.0, 359.9] {
            let rad = deg.to_radians();
            let decoded = decode_rotation(encode_rotation(rad));
            assert!(
                (decoded - rad).abs() < 0.0002,
                "rotation roundtrip failed for {deg}°: got {decoded}"
            );
        }
    }

    #[test]
    fn unknown_kind_returns_error() {
        assert!(matches!(
            decode(&[0xFF]),
            Err(DecodeError::UnknownKind(0xFF))
        ));
    }

    #[test]
    fn truncated_buffer_returns_eof() {
        let buf = encode(&AnyPacket::Ping(12345));
        // Drop the last byte.
        assert!(matches!(
            decode(&buf[..buf.len() - 1]),
            Err(DecodeError::UnexpectedEof)
        ));
    }
}
