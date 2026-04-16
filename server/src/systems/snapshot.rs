use std::net::SocketAddr;

use crate::session::{ServerState, Session};
use game::world::sight::Sight;
use game::world::wall::Wall;
use net::decode_rotation;
use net::proto::server::{BulletEvent, MatchState, PlayerState, SelfState, ServerPacket};

/// Build the server snapshot sent to one recipient this tick.
///
/// The snapshot contains:
/// - Authoritative self-state (position, health, ammo, rotation, aim cone).
/// - All remote players that are visible to the recipient (visibility-culled).
/// - All bullet events fired this tick (the client decides which to render).
/// - Current match state (timer, scores).
pub fn build_snapshot_for_player(
    tick: u32,
    bullets: &[BulletEvent],
    walls: &[Wall],
    st: &ServerState,
    recipient_addr: SocketAddr,
    recipient: &Session,
) -> ServerPacket {
    let me_x = recipient.x;
    let me_y = recipient.y;
    let me_rotation = recipient.rotation;
    let me_movement_state = recipient.movement_state;
    let me_weapon_stats = recipient.weapon.stats();

    // Build the recipient's sight cone for visibility culling remote players.
    let mut viewer_sight = Sight::player();
    viewer_sight.direction = decode_rotation(me_rotation);
    viewer_sight.range = me_weapon_stats.visibility_range;
    viewer_sight.half_angle = me_weapon_stats.visibility_half_angle_deg.to_radians();
    let viewer_pos = (me_x, me_y);

    let players: Vec<PlayerState> = st
        .sessions
        .iter()
        .filter_map(|(&addr, session)| {
            if addr == recipient_addr || session.health == 0 {
                return None;
            }
            if !viewer_sight.can_see(viewer_pos, (session.x, session.y), walls) {
                return None;
            }
            Some(PlayerState {
                id: session.player_id,
                x: session.x,
                y: session.y,
                rotation: session.rotation,
                movement_state: session.movement_state,
                weapon: 0,
            })
        })
        .collect();

    ServerPacket {
        tick,
        timestamp: recipient
            .latest_input
            .as_ref()
            .map(|pkt| pkt.timestamp)
            .unwrap_or(0),
        me: SelfState {
            health: recipient.health.min(u8::MAX as u16) as u8,
            ammo: recipient.weapon.ammo_in_mag().min(u8::MAX as u32) as u8,
            reload_progress: if recipient.weapon.is_reloading() {
                128
            } else {
                0
            },
            movement_state: me_movement_state,
            x: me_x,
            y: me_y,
            rotation: me_rotation,
            aim_cone_half_angle: recipient.aim_cone_half_angle,
        },
        players,
        // Every bullet event is sent to every client; the client's renderer
        // decides which ones are within local visibility.
        bullets: bullets.to_vec(),
        sounds: vec![],
        match_state: MatchState {
            timer: 300,
            score_team1: 0,
            score_team2: 0,
        },
    }
}
