use crate::camera::TacticalCamera;
use engine::input::InputState;
use game::game::Game;
use game::world::units::tiles_to_px;
use net::{ClientPacket, InputFlags, MoveSpeed, encode_rotation};

use crate::util::client_time_ms;

/// Build a `ClientPacket` from the current input state and player position.
///
/// The aim angle is computed in world space (mouse position → player position)
/// and encoded as a compressed u16 for transmission.
pub fn build_client_packet(
    seq: u16,
    input: &InputState,
    mouse_world: (f32, f32),
    game: &Game,
) -> ClientPacket {
    let mut flags = InputFlags::default();
    flags.set_shooting(input.shoot);
    flags.set_reloading(input.reload);
    flags.set_move_speed(if input.walk {
        MoveSpeed::SlowWalk
    } else if input.shift {
        MoveSpeed::Run
    } else {
        MoveSpeed::Walk
    });
    flags.set_peeking(input.peek);

    let angle =
        (mouse_world.1 - game.player.movement.y).atan2(mouse_world.0 - game.player.movement.x);

    ClientPacket {
        sequence: seq,
        timestamp: client_time_ms(),
        movement_x: (input.right as i8) - (input.left as i8),
        movement_y: (input.down as i8) - (input.up as i8),
        rotation: encode_rotation(angle),
        flags,
    }
}

/// Convert a raw screen-space mouse position to world-space tile coordinates
/// via the camera's inverse transform, then remap into the pixel coordinate
/// space expected by the game's player update (input stores pixel coords).
pub fn mouse_world_to_game_input(
    camera: &TacticalCamera,
    mouse_screen: (f32, f32),
    viewport_px: (f32, f32),
) -> (f64, f64) {
    let world = camera.screen_to_world(mouse_screen, viewport_px);
    (tiles_to_px(world.0) as f64, tiles_to_px(world.1) as f64)
}
