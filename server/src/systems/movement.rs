use crate::session::Session;
use game::systems::movement::{
    PEEK_DISTANCE, PEEK_LERP_SPEED, apply_actor_movement, clamped_peek_distance,
};
use game::world::level::LevelBounds;
use game::world::units::px_to_tiles;
use game::world::wall::Wall;
use net::proto::server::MovementState;
use net::{ClientPacket, MoveSpeed};

pub const PLAYER_HALF: f32 = px_to_tiles(10.0);
const WALK_SPEED: f32 = px_to_tiles(40.0);
const NORMAL_SPEED: f32 = px_to_tiles(85.0);
const RUN_SPEED: f32 = px_to_tiles(200.0);
const SPECTATOR_SPEED: f32 = RUN_SPEED;

/// Apply the latest client input to a session.
///
/// Spectators (dead players) get free-roam movement: no wall collision, direct
/// position update. Live players go through the normal physics path including
/// peek (X + WASD slides the player out from cover up to `PEEK_DISTANCE`).
pub fn apply_session_input(
    session: &mut Session,
    input: &ClientPacket,
    dt: f32,
    walls: &[Wall],
    level_bounds: Option<LevelBounds>,
) {
    if session.health == 0 {
        apply_spectator_movement(session, input, dt, level_bounds);
        return;
    }

    let peeking = input.flags.is_peeking();
    let px = input.movement_x as f32;
    let py = input.movement_y as f32;
    let has_dir = px != 0.0 || py != 0.0;

    if peeking && session.peek_origin.is_none() {
        session.peek_origin = Some((session.x, session.y));
    }

    if let Some(origin) = session.peek_origin {
        if !peeking && has_dir {
            // Released peek while moving — exit immediately, resume walking.
            session.peek_origin = None;
            apply_actor_movement(
                &mut session.x,
                &mut session.y,
                px,
                py,
                speed_from_input(input),
                dt,
                walls,
                PLAYER_HALF,
                level_bounds,
            );
        } else {
            let (target_x, target_y) = if peeking && has_dir {
                let len = (px * px + py * py).sqrt();
                let dir = (px / len, py / len);
                let dist = clamped_peek_distance(origin, dir, PEEK_DISTANCE, PLAYER_HALF, walls);
                (origin.0 + dir.0 * dist, origin.1 + dir.1 * dist)
            } else {
                origin
            };

            let alpha = (PEEK_LERP_SPEED * dt).min(1.0);
            session.x += (target_x - session.x) * alpha;
            session.y += (target_y - session.y) * alpha;

            if !peeking {
                let dx = session.x - origin.0;
                let dy = session.y - origin.1;
                if dx * dx + dy * dy < 1.0e-6 {
                    session.x = origin.0;
                    session.y = origin.1;
                    session.peek_origin = None;
                }
            }
        }
    } else {
        apply_actor_movement(
            &mut session.x,
            &mut session.y,
            px,
            py,
            speed_from_input(input),
            dt,
            walls,
            PLAYER_HALF,
            level_bounds,
        );
    }

    session.rotation = input.rotation;
    session.movement_state = movement_state_from_input(Some(input), false);
}

fn apply_spectator_movement(
    session: &mut Session,
    input: &ClientPacket,
    dt: f32,
    level_bounds: Option<LevelBounds>,
) {
    let vx = input.movement_x as f32;
    let vy = input.movement_y as f32;
    let len = (vx * vx + vy * vy).sqrt();
    if len > 0.001 {
        session.x += (vx / len) * SPECTATOR_SPEED * dt;
        session.y += (vy / len) * SPECTATOR_SPEED * dt;
    }
    if let Some(b) = level_bounds {
        session.x = session.x.clamp(b.x, b.x + b.w);
        session.y = session.y.clamp(b.y, b.y + b.h);
    }
    session.rotation = input.rotation;
    session.movement_state = MovementState(0);
}

/// Derive the movement state bitfield from a client input packet.
///
/// Passing `None` for `latest_input` yields a zeroed (walking, not reloading)
/// state, which is the safe default for newly connected sessions.
pub fn movement_state_from_input(
    latest_input: Option<&ClientPacket>,
    is_reloading: bool,
) -> MovementState {
    let mut state = MovementState(0);
    state.set_move_speed(
        latest_input
            .map(|pkt| pkt.flags.move_speed())
            .unwrap_or(MoveSpeed::Walk),
    );
    state.set_peeking(
        latest_input
            .map(|pkt| pkt.flags.is_peeking())
            .unwrap_or(false),
    );
    state.set_reloading(is_reloading);
    state
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

fn speed_from_input(input: &ClientPacket) -> f32 {
    match input.flags.move_speed() {
        MoveSpeed::SlowWalk => WALK_SPEED,
        MoveSpeed::Walk => NORMAL_SPEED,
        MoveSpeed::Run => RUN_SPEED,
    }
}
