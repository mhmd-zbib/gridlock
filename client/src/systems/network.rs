use crate::net::NetClient;
use crate::render::entities::NetBulletTrace;
use net::{MatchState, PlayerState, SelfState, TeammateView};

/// Drain all pending server snapshots from the network queue and apply them to
/// the provided state slices.
pub fn apply_server_snapshots(
    net: &NetClient,
    server_me: &mut Option<SelfState>,
    net_players: &mut Vec<PlayerState>,
    teammate_sight_cones: &mut Vec<TeammateView>,
    net_bullet_traces: &mut Vec<NetBulletTrace>,
    match_state: &mut Option<MatchState>,
    my_team: u8,
) {
    use crate::render::entities::NET_BULLET_TTL;

    for snap in net.take_snapshots() {
        *server_me = Some(snap.me);
        *net_players = snap.players;
        *teammate_sight_cones = snap.teammate_views;
        *match_state = Some(snap.match_state);
        for b in snap.bullets {
            let friendly = my_team != 0 && b.shooter_team == my_team;
            net_bullet_traces.push(NetBulletTrace {
                from_x: b.from_x,
                from_y: b.from_y,
                to_x: b.to_x,
                to_y: b.to_y,
                ttl: NET_BULLET_TTL,
                friendly,
            });
        }
    }
}
