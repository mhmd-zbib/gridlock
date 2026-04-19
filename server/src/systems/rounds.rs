use crate::session::{RESPAWN_DELAY, ServerState};
use crate::util::spawn_offset;

const MAX_HEALTH: u16 = 100;

/// Called every tick while a match is in progress.
///
/// Responsibilities:
/// - Detect when one team has been fully eliminated and start the respawn
///   countdown, crediting the surviving team with a round win.
/// - Tick down the respawn countdown and, when it expires, reset everyone's
///   health and teleport them back to their team spawn points.
///   Mid-game spectators with a team assigned are also spawned in at this point.
pub fn step_rounds(st: &mut ServerState, dt: f32) {
    if st.respawn_timer > 0.0 {
        st.respawn_timer -= dt;
        if st.respawn_timer <= 0.0 {
            st.respawn_timer = 0.0;
            respawn_all(st);
        }
        return;
    }

    // Only active (non-spectator) players count for round-end detection.
    let (alive1, total1, alive2, total2) = count_active_teams(st);

    // Need at least one active player on each side before declaring round over.
    if total1 == 0 || total2 == 0 {
        return;
    }

    if alive1 == 0 && alive2 > 0 {
        st.team2_rounds = st.team2_rounds.saturating_add(1);
        st.respawn_timer = RESPAWN_DELAY;
    } else if alive2 == 0 && alive1 > 0 {
        st.team1_rounds = st.team1_rounds.saturating_add(1);
        st.respawn_timer = RESPAWN_DELAY;
    } else if alive1 == 0 && alive2 == 0 {
        // Simultaneous wipe — no point awarded.
        st.respawn_timer = RESPAWN_DELAY;
    }
}

/// Count only players who are actively playing (not spectating mid-game joiners).
fn count_active_teams(st: &ServerState) -> (u32, u32, u32, u32) {
    let (mut alive1, mut total1, mut alive2, mut total2) = (0u32, 0u32, 0u32, 0u32);
    for s in st.sessions.values() {
        if s.is_spectator {
            continue; // mid-game joiners don't count toward round totals
        }
        match s.team {
            1 => {
                total1 += 1;
                if s.health > 0 {
                    alive1 += 1;
                }
            }
            2 => {
                total2 += 1;
                if s.health > 0 {
                    alive2 += 1;
                }
            }
            _ => {}
        }
    }
    (alive1, total1, alive2, total2)
}

fn respawn_all(st: &mut ServerState) {
    let team1_spawn = st.team1_spawn;
    let team2_spawn = st.team2_spawn;
    let default_spawn = st.spawn;
    for session in st.sessions.values_mut() {
        // Players with no team stay as spectators across rounds.
        if session.team == 0 {
            continue;
        }
        let base = match session.team {
            1 => team1_spawn.unwrap_or(default_spawn),
            2 => team2_spawn.unwrap_or(default_spawn),
            _ => default_spawn,
        };
        let offset = spawn_offset(session.player_id);
        session.x = base.0 + offset.0;
        session.y = base.1 + offset.1;
        session.health = MAX_HEALTH;
        session.peek_origin = None;
        // Mid-game spectators who have a team now join the fight.
        session.is_spectator = false;
    }
}
