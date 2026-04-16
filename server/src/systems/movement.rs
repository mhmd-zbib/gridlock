use crate::session::Session;
use game::world::level::LevelBounds;
use game::world::units::px_to_tiles;
use game::world::wall::{self, Wall};
use net::proto::server::MovementState;
use net::{ClientPacket, MoveSpeed};

const PLAYER_HALF: f32 = px_to_tiles(10.0);
const WALK_SPEED: f32 = px_to_tiles(40.0);
const NORMAL_SPEED: f32 = px_to_tiles(85.0);
const RUN_SPEED: f32 = px_to_tiles(200.0);

/// Apply the latest client input to a session: translate position, resolve
/// wall collisions, clamp to level bounds, and update movement/rotation state.
pub fn apply_session_input(
    session: &mut Session,
    input: &ClientPacket,
    dt: f32,
    walls: &[Wall],
    level_bounds: Option<LevelBounds>,
) {
    let speed = speed_from_input(input);
    let mut dx = input.movement_x as f32;
    let mut dy = input.movement_y as f32;
    let len = (dx * dx + dy * dy).sqrt();
    if len > 1.0 {
        dx /= len;
        dy /= len;
    }

    session.x += dx * speed * dt;
    session.y += dy * speed * dt;
    wall::resolve_all(&mut session.x, &mut session.y, PLAYER_HALF, walls);
    clamp_to_bounds(&mut session.x, &mut session.y, PLAYER_HALF, level_bounds);

    session.rotation = input.rotation;
    session.movement_state = movement_state_from_input(Some(input), false);
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

fn clamp_to_bounds(x: &mut f32, y: &mut f32, half: f32, bounds: Option<LevelBounds>) {
    let Some(bounds) = bounds else { return };
    let min_x = bounds.x + half;
    let max_x = bounds.x + bounds.w - half;
    let min_y = bounds.y + half;
    let max_y = bounds.y + bounds.h - half;

    if min_x <= max_x {
        *x = x.clamp(min_x, max_x);
    } else {
        *x = bounds.x + bounds.w * 0.5;
    }
    if min_y <= max_y {
        *y = y.clamp(min_y, max_y);
    } else {
        *y = bounds.y + bounds.h * 0.5;
    }
}
