use crate::camera::TacticalCamera;
use crate::render::sight_geometry::{circle_arc_pts_raw, cone_arc_pts_raw};
use crate::render::views::FogView;
use game::render::geometry::{GeoVertex, push_cone_fan};
use game::world::wall::Wall;

/// Build the stencil-buffer geometry that defines the player's visible region.
/// Includes the union of all teammate cones for shared-vision fog-of-war.
pub fn vision_cone_mask(
    view: &FogView<'_>,
    camera: &TacticalCamera,
    viewport_px: (f32, f32),
) -> Vec<GeoVertex> {
    let mut out = Vec::new();

    // Local player cone.
    add_cone_to_mask(
        &mut out,
        camera,
        viewport_px,
        view.player_pos,
        view.walls,
        view.sight_direction,
        view.sight_half_angle,
        view.sight_range,
        view.sight_circle_radius,
    );

    // Teammate cones — unioned into the same stencil mask.
    for tm in &view.teammate_cones {
        add_cone_to_mask(
            &mut out,
            camera,
            viewport_px,
            tm.pos,
            view.walls,
            tm.sight_direction,
            tm.sight_half_angle,
            tm.sight_range,
            tm.sight_circle_radius,
        );
    }

    out
}

fn add_cone_to_mask(
    out: &mut Vec<GeoVertex>,
    camera: &TacticalCamera,
    viewport_px: (f32, f32),
    pos: (f32, f32),
    walls: &[Wall],
    direction: f32,
    half_angle: f32,
    range: f32,
    circle_radius: f32,
) {
    let center_px = camera.world_to_screen(pos, viewport_px);

    let circle = camera.world_points_to_screen(
        circle_arc_pts_raw(pos, walls, 64, circle_radius),
        viewport_px,
    );
    push_cone_fan(out, center_px, &circle, [1.0, 1.0, 1.0, 1.0]);

    let arc = camera.world_points_to_screen(
        cone_arc_pts_raw(pos, walls, 60, direction, half_angle, range),
        viewport_px,
    );
    push_cone_fan(out, center_px, &arc, [1.0, 1.0, 1.0, 1.0]);
}
