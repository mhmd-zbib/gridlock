use std::net::SocketAddr;

use crate::session::{ServerState, Session};
use game::world::sight::Sight;
use game::world::wall::Wall;
use net::decode_rotation;
use net::proto::server::{BulletEvent, MatchState, PlayerState, SelfState, ServerPacket, TeammateView};

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

    // Spectators (mid-game joiners or dead players waiting for respawn) see
    // every living player — skip all visibility culling for them.
    let recipient_is_spectator = recipient.is_spectator || recipient.health == 0;

    let my_team = recipient.team;

    // Build teammate sight cones so the client can union them into the fog mask.
    let mut teammate_views: Vec<TeammateView> = Vec::new();
    if my_team != 0 && !recipient_is_spectator {
        for (&addr, session) in st.sessions.iter() {
            if addr == recipient_addr || session.health == 0 || session.team != my_team {
                continue;
            }
            let mut tm_sight = Sight::player();
            tm_sight.direction = decode_rotation(session.rotation);
            let tm_stats = session.weapon.stats();
            tm_sight.range = tm_stats.visibility_range;
            tm_sight.half_angle = tm_stats.visibility_half_angle_deg.to_radians();
            teammate_views.push(TeammateView {
                x: session.x,
                y: session.y,
                rotation: session.rotation,
                sight_range: tm_sight.range,
                sight_half_angle: tm_sight.half_angle,
                sight_circle_radius: tm_sight.circle_radius,
            });
        }
    }

    let players: Vec<PlayerState> = st
        .sessions
        .iter()
        .filter_map(|(&addr, session)| {
            if addr == recipient_addr || session.health == 0 {
                return None;
            }
            if !recipient_is_spectator {
                let is_teammate = my_team != 0 && session.team == my_team;
                // Teammates are always visible; enemies require LOS through the
                // recipient's cone or any teammate's cone.
                if !is_teammate {
                    let visible_to_me = viewer_sight.can_see(viewer_pos, (session.x, session.y), walls);
                    let visible_to_team = teammate_views.iter().any(|tv| {
                        let mut tm_sight = Sight::player();
                        tm_sight.direction = decode_rotation(tv.rotation);
                        tm_sight.range = tv.sight_range;
                        tm_sight.half_angle = tv.sight_half_angle;
                        tm_sight.circle_radius = tv.sight_circle_radius;
                        tm_sight.can_see((tv.x, tv.y), (session.x, session.y), walls)
                    });
                    if !visible_to_me && !visible_to_team {
                        return None;
                    }
                }
            }
            Some(PlayerState {
                id: session.player_id,
                x: session.x,
                y: session.y,
                rotation: session.rotation,
                movement_state: session.movement_state,
                weapon: 0,
                team: session.team,
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
            aim_cone_half_angle: recipient.aim_cone.half_angle(),
        },
        players,
        // Every bullet event is sent to every client; the client's renderer
        // decides which ones are within local visibility.
        bullets: bullets.to_vec(),
        sounds: vec![],
        teammate_views,
        match_state: MatchState {
            timer: st.respawn_timer.ceil() as u16,
            score_team1: st.team1_rounds,
            score_team2: st.team2_rounds,
        },
    }
}
