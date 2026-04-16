use crate::net::NetClient;
use crate::render::entities::NetBulletTrace;
use net::{PlayerState, SelfState};

/// Drain all pending server snapshots from the network queue and apply them to
/// the provided state slices.
///
/// Every snapshot is drained (not just the latest) because bullet events exist
/// only in the tick they were fired — dropping intermediate snapshots would
/// miss bullet traces from ticks that were queued but not yet consumed.
///
/// After this call:
/// - `server_me` holds the most recent authoritative self-state.
/// - `net_players` holds the most recent remote player list.
/// - New bullet traces are appended to `net_bullet_traces` with full lifetime.
pub fn apply_server_snapshots(
    net: &NetClient,
    server_me: &mut Option<SelfState>,
    net_players: &mut Vec<PlayerState>,
    net_bullet_traces: &mut Vec<NetBulletTrace>,
) {
    use crate::render::entities::NET_BULLET_TTL;

    for snap in net.take_snapshots() {
        *server_me = Some(snap.me);
        *net_players = snap.players;
        for b in snap.bullets {
            net_bullet_traces.push(NetBulletTrace {
                from_x: b.from_x,
                from_y: b.from_y,
                to_x: b.to_x,
                to_y: b.to_y,
                ttl: NET_BULLET_TTL,
            });
        }
    }
}
