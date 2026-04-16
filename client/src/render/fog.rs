use engine::render::geometry::{GeoVertex, push_cone_fan};
use game::game::Game;
use game::world::camera::TacticalCamera;
use game::world::sight::Sight;
use net::SelfState;
use net::decode_rotation;

/// Build the stencil-buffer geometry that defines the player's visible region.
///
/// The resulting triangle fans are written into the stencil buffer (no colour
/// output).  The quad and geometry passes then use this mask to dim or hide
/// everything outside the player's vision cone.
///
/// When the server has sent an authoritative rotation (`server_me`) that
/// rotation is used instead of the locally predicted one so the cone direction
/// stays in sync with the server.
pub fn vision_cone_mask(
    game: &Game,
    camera: &TacticalCamera,
    viewport_px: (f32, f32),
    server_me: Option<&SelfState>,
) -> Vec<GeoVertex> {
    let walls = &game.walls;
    let player_pos = (game.player.movement.x, game.player.movement.y);
    let player_px = camera.world_to_screen(player_pos, viewport_px);

    // Prefer server-authoritative rotation; fall back to local prediction.
    let mut sight = Sight::player();
    sight.direction = server_me
        .map(|me| decode_rotation(me.rotation))
        .unwrap_or(game.player.sight.direction);
    sight.half_angle = game.player.sight.half_angle;
    sight.range = game.player.sight.range;
    sight.circle_radius = game.player.sight.circle_radius;

    let mut out = Vec::new();

    // Close-range circle (occluded by walls).
    let circle =
        camera.world_points_to_screen(sight.circle_arc_pts(player_pos, walls, 64), viewport_px);
    push_cone_fan(&mut out, player_px, &circle, [1.0, 1.0, 1.0, 1.0]);

    // Directional cone (occluded by walls).
    let arc = camera.world_points_to_screen(sight.cone_arc_pts(player_pos, walls, 60), viewport_px);
    push_cone_fan(&mut out, player_px, &arc, [1.0, 1.0, 1.0, 1.0]);

    out
}
