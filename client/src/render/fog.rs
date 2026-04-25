use crate::camera::TacticalCamera;
use crate::render::views::FogView;
use engine::render::fog::{FogSource, FogUniforms, MAX_FOG_SOURCES};
use engine::render::geometry::{GeoVertex, push_cone_fan};
use game::world::wall::Wall;

use super::sight_geometry::{circle_arc_pts_raw, cone_arc_pts_raw};

/// Build the stencil-buffer geometry that defines the player's visible region.
/// Includes the union of all teammate cones for shared-vision fog-of-war.
pub fn vision_cone_mask(
    view: &FogView<'_>,
    camera: &TacticalCamera,
    viewport_px: (f32, f32),
) -> Vec<GeoVertex> {
    let mut out = Vec::new();

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

/// Build the fog uniform block that drives the per-pixel spotlight shader.
///
/// The shader computes a mathematically correct illumination model per fragment:
/// - An omnidirectional near-halo with a Gaussian-like squared-distance falloff.
/// - A directional cone with Perlin smootherstep (C²) angular feathering and a
///   cubic distance attenuation that stays at full brightness for the first 40%
///   of the cone range before falling off smoothly.
///
/// Multiple sources (local player + teammates) are composited by max() so that
/// any illuminated area from any cone contributes to the final brightness.
pub fn vision_cone_fog(
    view: &FogView<'_>,
    camera: &TacticalCamera,
    viewport_px: (f32, f32),
    outside_dim: f32,
) -> FogUniforms {
    let transform = camera.screen_transform(viewport_px);
    let ppu = transform.pixels_per_unit;
    let mut uniforms = FogUniforms::new([viewport_px.0, viewport_px.1], outside_dim);

    // Local player — skip when spectating (sight_range == 0 && circle == 0).
    let is_spectating = view.sight_range == 0.0 && view.sight_circle_radius == 0.0;
    if !is_spectating {
        let (sx, sy) = transform.world_to_screen(view.player_pos);
        let dir = [view.sight_direction.cos(), view.sight_direction.sin()];
        uniforms.push(FogSource::new(
            [sx, sy],
            view.sight_range * ppu,
            view.sight_circle_radius * ppu,
            dir,
            view.sight_half_angle,
        ));
    }

    // Teammates — union their cones into the same fog pass (capped at MAX_FOG_SOURCES).
    let remaining = MAX_FOG_SOURCES - uniforms.source_count as usize;
    for tm in view.teammate_cones.iter().take(remaining) {
        let (sx, sy) = transform.world_to_screen(tm.pos);
        let dir = [tm.sight_direction.cos(), tm.sight_direction.sin()];
        uniforms.push(FogSource::new(
            [sx, sy],
            tm.sight_range * ppu,
            tm.sight_circle_radius * ppu,
            dir,
            tm.sight_half_angle,
        ));
    }

    uniforms
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
