use crate::camera::TacticalCamera;
use crate::render::sight_geometry::{circle_arc_pts_raw, cone_arc_pts_raw};
use crate::render::views::FogView;
use engine::render::geometry::{GeoVertex, push_cone_fan};

/// Build the stencil-buffer geometry that defines the player's visible region.
pub fn vision_cone_mask(
    view: &FogView<'_>,
    camera: &TacticalCamera,
    viewport_px: (f32, f32),
) -> Vec<GeoVertex> {
    let player_px = camera.world_to_screen(view.player_pos, viewport_px);
    let mut out = Vec::new();

    let circle = camera.world_points_to_screen(
        circle_arc_pts_raw(view.player_pos, view.walls, 64, view.sight_circle_radius),
        viewport_px,
    );
    push_cone_fan(&mut out, player_px, &circle, [1.0, 1.0, 1.0, 1.0]);

    let arc = camera.world_points_to_screen(
        cone_arc_pts_raw(
            view.player_pos,
            view.walls,
            60,
            view.sight_direction,
            view.sight_half_angle,
            view.sight_range,
        ),
        viewport_px,
    );
    push_cone_fan(&mut out, player_px, &arc, [1.0, 1.0, 1.0, 1.0]);

    out
}
