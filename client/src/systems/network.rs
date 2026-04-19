use crate::net::NetClient;
use crate::render::entities::NetBulletTrace;
use net::{MatchState, PlayerState, SelfState, TeammateView};

pub struct SnapshotApplyResult {
    pub killer: Option<String>,
    pub latest_ack_timestamp: Option<u32>,
}

/// Drain all pending server snapshots from the network queue and apply them to
/// the provided state slices.
///
/// Returns the killer's display name (if this tick killed the local player)
/// and the latest echoed input timestamp ack from the server.
pub fn apply_server_snapshots(
    net: &NetClient,
    server_me: &mut Option<SelfState>,
    net_players: &mut Vec<PlayerState>,
    teammate_sight_cones: &mut Vec<TeammateView>,
    net_bullet_traces: &mut Vec<NetBulletTrace>,
    match_state: &mut Option<MatchState>,
    last_server_tick: &mut Option<u32>,
    my_team: u8,
    my_player_id: Option<u16>,
) -> SnapshotApplyResult {
    use crate::render::entities::NET_BULLET_TTL;

    let mut killer: Option<String> = None;
    let mut latest_ack_timestamp: Option<u32> = None;

    for snap in net.take_snapshots() {
        if last_server_tick.is_some_and(|tick| snap.tick <= tick) {
            continue;
        }
        *last_server_tick = Some(snap.tick);
        latest_ack_timestamp = Some(snap.timestamp);
        *server_me = Some(snap.me);
        *net_players = snap.players;
        *teammate_sight_cones = snap.teammate_views;
        *match_state = Some(snap.match_state);
        for b in snap.bullets {
            if let Some(pid) = my_player_id {
                if b.hit_player_id == pid && pid != 0 {
                    killer = Some(b.shooter_name_str().to_owned());
                }
            }
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

    SnapshotApplyResult {
        killer,
        latest_ack_timestamp,
    }
}
