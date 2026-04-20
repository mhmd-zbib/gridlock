use super::{error::DecodeError, io::BufReader};
use crate::proto::{
    client::{ClientPacket, InputFlags},
    handshake::{AnyPacket, ConnectAck, ConnectRequest, ConnectResult, PacketKind},
    lobby::{LobbyCommand, LobbyCommandKind, LobbyPlayer, LobbyState},
    server::{
        BulletEvent, MatchState, MovementState, PlayerState, SelfState, ServerPacket, SoundEvent,
        SoundKind, TeammateView,
    },
};

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
            let shot_seed = r.u32()?;
            AnyPacket::ClientInput(ClientPacket {
                sequence,
                timestamp,
                movement_x,
                movement_y,
                rotation,
                flags,
                shot_seed,
            })
        }
        PacketKind::ServerSnapshot => AnyPacket::ServerSnapshot(decode_snapshot(&mut r)?),
        PacketKind::LobbyCommand => {
            let kind = LobbyCommandKind::from_u8(r.u8()?)
                .ok_or(DecodeError::InvalidField("lobby_command_kind"))?;
            let team = r.u8()?;
            AnyPacket::LobbyCommand(LobbyCommand { kind, team })
        }
        PacketKind::LobbyState => {
            let game_started = r.u8()? != 0;
            let your_team = r.u8()?;
            let team1_count = r.u8()?;
            let team2_count = r.u8()?;
            let is_creator = r.u8()? != 0;
            let player_count = r.u8()? as usize;
            let mut players = Vec::with_capacity(player_count);
            for _ in 0..player_count {
                let mut name = [0u8; 16];
                name.copy_from_slice(r.bytes(16)?);
                let team = r.u8()?;
                players.push(LobbyPlayer { name, team });
            }
            AnyPacket::LobbyState(LobbyState {
                game_started,
                your_team,
                team1_count,
                team2_count,
                is_creator,
                players,
            })
        }
        PacketKind::Disconnect => AnyPacket::Disconnect,
        PacketKind::Ping => AnyPacket::Ping(r.u32()?),
        PacketKind::Pong => AnyPacket::Pong(r.u32()?),
    };

    Ok(packet)
}

fn decode_snapshot(r: &mut BufReader<'_>) -> Result<ServerPacket, DecodeError> {
    let tick = r.u32()?;
    let timestamp = r.u32()?;
    let me = SelfState {
        health: r.u8()?,
        ammo: r.u8()?,
        reload_progress: r.u8()?,
        movement_state: MovementState(r.u8()?),
        x: r.f32()?,
        y: r.f32()?,
        rotation: r.u16()?,
        aim_cone_half_angle: r.f32()?,
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
            team: r.u8()?,
        });
    }

    let bullet_count = r.u8()? as usize;
    let mut bullets = Vec::with_capacity(bullet_count);
    for _ in 0..bullet_count {
        let shooter_id = r.u16()?;
        let from_x = r.f32()?;
        let from_y = r.f32()?;
        let to_x = r.f32()?;
        let to_y = r.f32()?;
        let hit_player_id = r.u16()?;
        let shooter_team = r.u8()?;
        let mut shooter_name = [0u8; 16];
        shooter_name.copy_from_slice(r.bytes(16)?);
        bullets.push(BulletEvent {
            shooter_id,
            from_x,
            from_y,
            to_x,
            to_y,
            hit_player_id,
            shooter_team,
            shooter_name,
        });
    }

    let sound_count = r.u8()? as usize;
    let mut sounds = Vec::with_capacity(sound_count);
    for _ in 0..sound_count {
        let kind_byte = r.u8()?;
        let kind = SoundKind::from_u8(kind_byte).ok_or(DecodeError::InvalidField("sound_kind"))?;
        sounds.push(SoundEvent {
            kind,
            x: r.f32()?,
            y: r.f32()?,
            intensity: r.u8()?,
        });
    }

    let teammate_count = r.u8()? as usize;
    let mut teammate_views = Vec::with_capacity(teammate_count);
    for _ in 0..teammate_count {
        teammate_views.push(TeammateView {
            x: r.f32()?,
            y: r.f32()?,
            rotation: r.u16()?,
            sight_range: r.f32()?,
            sight_half_angle: r.f32()?,
            sight_circle_radius: r.f32()?,
        });
    }

    let match_state = MatchState {
        timer: r.u16()?,
        score_team1: r.u8()?,
        score_team2: r.u8()?,
    };

    Ok(ServerPacket {
        tick,
        timestamp,
        me,
        players,
        bullets,
        sounds,
        teammate_views,
        match_state,
    })
}

/// Expand a wire-format rotation back to radians in `[0, 2π)`.
#[inline]
pub fn decode_rotation(v: u16) -> f32 {
    (v as f32 / 65535.0) * std::f32::consts::TAU
}
