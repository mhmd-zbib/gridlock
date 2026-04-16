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

mod decode;
mod encode;
mod error;
mod io;

pub use decode::{decode, decode_rotation};
pub use encode::{encode, encode_rotation};
pub use error::DecodeError;

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::client::ClientPacket;
    use crate::proto::handshake::AnyPacket;
    use crate::proto::server::{
        BulletEvent, MatchState, MovementState, PlayerState, SelfState, ServerPacket, SoundEvent,
    };
    use crate::proto::{
        client::InputFlags,
        handshake::{ConnectRequest, PROTOCOL_VERSION},
        lobby::{LobbyCommand, LobbyCommandKind, LobbyState},
        server::SoundKind,
    };

    #[test]
    fn roundtrip_connect_request() {
        let original = AnyPacket::ConnectRequest(ConnectRequest::new("TestPlayer"));
        let buf = encode(&original);
        let decoded = decode(&buf).unwrap();
        let AnyPacket::ConnectRequest(p) = decoded else {
            panic!("wrong variant")
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
            rotation: encode_rotation(1.5707964),
            flags,
        });
        let buf = encode(&original);
        let decoded = decode(&buf).unwrap();
        let AnyPacket::ClientInput(p) = decoded else {
            panic!("wrong variant")
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
                x: 19.125,
                y: 28.75,
                rotation: encode_rotation(1.0),
                aim_cone_half_angle: 0.125,
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
            panic!("wrong variant")
        };
        assert_eq!(p.tick, 42);
        assert_eq!(p.me.health, 100);
        assert!((p.me.x - 19.125).abs() < 1e-5);
        assert!((p.me.y - 28.75).abs() < 1e-5);
        assert!((p.me.aim_cone_half_angle - 0.125).abs() < 1e-6);
        assert_eq!(p.players.len(), 1);
        assert_eq!(p.players[0].id, 7);
        assert!((p.players[0].x - 3.5).abs() < 1e-5);
        assert_eq!(p.bullets.len(), 1);
        assert_eq!(p.sounds.len(), 1);
        assert_eq!(p.sounds[0].kind, SoundKind::Gunshot);
        assert_eq!(p.match_state.timer, 180);
    }

    #[test]
    fn roundtrip_lobby_packets() {
        let select = AnyPacket::LobbyCommand(LobbyCommand::select_team(2));
        let decoded = decode(&encode(&select)).unwrap();
        let AnyPacket::LobbyCommand(cmd) = decoded else {
            panic!("wrong variant")
        };
        assert_eq!(cmd.kind, LobbyCommandKind::SelectTeam);
        assert_eq!(cmd.team, 2);

        let state = AnyPacket::LobbyState(LobbyState {
            game_started: true,
            your_team: 1,
            team1_count: 2,
            team2_count: 0,
        });
        let decoded = decode(&encode(&state)).unwrap();
        let AnyPacket::LobbyState(lobby) = decoded else {
            panic!("wrong variant")
        };
        assert!(lobby.game_started);
        assert_eq!(lobby.your_team, 1);
        assert_eq!(lobby.team1_count, 2);
        assert_eq!(lobby.team2_count, 0);
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
        assert!(matches!(
            decode(&buf[..buf.len() - 1]),
            Err(DecodeError::UnexpectedEof)
        ));
    }
}
