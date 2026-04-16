use super::io::BufWriter;
use crate::proto::{
    client::ClientPacket,
    handshake::{AnyPacket, PacketKind},
    lobby::{LobbyCommand, LobbyState},
    server::{ServerPacket, SoundEvent},
};

/// Serialise `packet` into a byte vector ready for the socket layer.
/// The first byte is always the [`PacketKind`] discriminant.
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
        AnyPacket::ServerSnapshot(p) => encode_snapshot(&mut w, p),
        AnyPacket::LobbyCommand(p) => encode_lobby_command(&mut w, p),
        AnyPacket::LobbyState(p) => encode_lobby_state(&mut w, p),
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

fn encode_snapshot(w: &mut BufWriter, p: &ServerPacket) {
    w.u8(PacketKind::ServerSnapshot as u8);
    w.u32(p.tick);
    w.u32(p.timestamp);
    // self state
    w.u8(p.me.health);
    w.u8(p.me.ammo);
    w.u8(p.me.reload_progress);
    w.u8(p.me.movement_state.0);
    w.f32(p.me.x);
    w.f32(p.me.y);
    w.u16(p.me.rotation);
    w.f32(p.me.aim_cone_half_angle);
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
        encode_sound(w, s);
    }
    // match state
    w.u16(p.match_state.timer);
    w.u8(p.match_state.score_team1);
    w.u8(p.match_state.score_team2);
}

fn encode_sound(w: &mut BufWriter, s: &SoundEvent) {
    w.u8(s.kind as u8);
    w.f32(s.x);
    w.f32(s.y);
    w.u8(s.intensity);
}

fn encode_lobby_command(w: &mut BufWriter, p: &LobbyCommand) {
    w.u8(PacketKind::LobbyCommand as u8);
    w.u8(p.kind as u8);
    w.u8(p.team);
}

fn encode_lobby_state(w: &mut BufWriter, p: &LobbyState) {
    w.u8(PacketKind::LobbyState as u8);
    w.u8(p.game_started as u8);
    w.u8(p.your_team);
    w.u8(p.team1_count);
    w.u8(p.team2_count);
}

/// Compress a radian angle to a `u16` for wire transmission.
///
/// Any input is normalised to `[0, 2π)` before encoding.
#[inline]
pub fn encode_rotation(rad: f32) -> u16 {
    let normalised = rad.rem_euclid(std::f32::consts::TAU);
    ((normalised / std::f32::consts::TAU) * 65535.0) as u16
}
