use crate::session::Session;
use game::systems::movement::apply_actor_movement;
use game::world::level::LevelBounds;
use game::world::units::px_to_tiles;
use game::world::wall::Wall;
use net::proto::server::MovementState;
use net::{ClientPacket, MoveSpeed};

pub const PLAYER_HALF: f32 = px_to_tiles(10.0);
const WALK_SPEED: f32 = px_to_tiles(40.0);
const NORMAL_SPEED: f32 = px_to_tiles(85.0);
const RUN_SPEED: f32 = px_to_tiles(200.0);

/// Apply the latest client input to a session.
///
/// Movement physics (normalisation, wall collision, bounds clamping) are
/// delegated to `game::systems::movement::apply_actor_movement` so that the
/// server never duplicates physics logic — game is the single source of truth.
pub fn apply_session_input(
    session: &mut Session,
    input: &ClientPacket,
    dt: f32,
    walls: &[Wall],
    level_bounds: Option<LevelBounds>,
) {
    apply_actor_movement(
        &mut session.x,
        &mut session.y,
        input.movement_x as f32,
        input.movement_y as f32,
        speed_from_input(input),
        dt,
        walls,
        PLAYER_HALF,
        level_bounds,
    );
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

