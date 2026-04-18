use crate::camera::TacticalCamera;
use crate::render::entities::{NET_BULLET_TTL, NetBulletTrace};
use crate::render::sight_geometry::{aim_cone_arc_pts, circle_arc_pts_raw, cone_arc_pts_raw};
use crate::render::views::{DebugRoomsView, GeometryView};
use game::render::geometry::{
    GeoVertex, push_circle_fan, push_cone_fan, push_diamond, push_line_segment, push_rect,
};

/// Alpha of the black overlay applied outside the player's vision cone.
pub const OUTSIDE_CONE_DIM: f32 = 0.9;

/// Build all geometry overlays for the play state.
///
/// Returns `(scene_geo, masked_geo)`:
/// - `scene_geo` — always rendered (player cone outlines, optional debug rooms).
/// - `masked_geo` — hidden outside the vision cone (enemy cones, bullet traces,
///   impact marks).
pub fn play_geometry(
    view: &GeometryView<'_>,
    camera: &TacticalCamera,
    viewport_px: (f32, f32),
    net_bullets: &[NetBulletTrace],
) -> (Vec<GeoVertex>, Vec<GeoVertex>) {
    let mut scene = Vec::new();
    let mut masked = Vec::new();
    let walls = view.walls;
    let player_pos = view.player_sight.pos;
    let player_px = camera.world_to_screen(player_pos, viewport_px);

    // Debug: room / gap topology overlay.
    if let Some(rooms) = &view.debug_rooms {
        push_rooms_debug(&mut scene, rooms, camera, viewport_px);
    }

    // Player sight circle and cone (semi-transparent fill).
    let circle = camera.world_points_to_screen(
        circle_arc_pts_raw(player_pos, walls, 64, view.player_sight.circle_radius),
        viewport_px,
    );
    push_cone_fan(&mut scene, player_px, &circle, [0.3, 0.7, 1.0, 0.07]);

    let arc = camera.world_points_to_screen(
        cone_arc_pts_raw(
            player_pos,
            walls,
            60,
            view.player_sight.direction,
            view.player_sight.half_angle,
            view.player_sight.range,
        ),
        viewport_px,
    );
    push_cone_fan(&mut scene, player_px, &arc, [0.3, 0.7, 1.0, 0.16]);

    // Aim cone (orange, wall-clipped).
    let aim_arc = camera.world_points_to_screen(
        aim_cone_arc_pts(
            player_pos,
            walls,
            16,
            view.aim_cone.direction,
            view.aim_cone.half_angle,
            view.aim_cone.render_range,
        ),
        viewport_px,
    );
    push_cone_fan(&mut scene, player_px, &aim_arc, [1.0, 0.6, 0.1, 0.45]);

    // Bullet impact marks.
    for impact in &view.impacts {
        let pos = camera.world_to_screen(impact.pos, viewport_px);
        push_circle_fan(
            &mut masked,
            pos,
            5.0,
            [1.0, 0.95, 0.2, 0.22 * impact.alpha],
            18,
        );
    }

    // Enemy sight circles and cones.
    for cone in &view.enemy_cones {
        let ep_px = camera.world_to_screen(cone.pos, viewport_px);

        push_circle_fan(
            &mut masked,
            ep_px,
            cone.circle_radius_px,
            cone.circle_color,
            36,
        );

        let arc = camera.world_points_to_screen(
            cone_arc_pts_raw(
                cone.pos,
                walls,
                48,
                cone.sight_direction,
                cone.sight_half_angle,
                cone.sight_range,
            ),
            viewport_px,
        );
        push_cone_fan(&mut masked, ep_px, &arc, cone.cone_color);
    }

    // Server-authoritative bullet tracers.
    let max_trace_px = {
        let (sw, sh) = viewport_px;
        (sw * sw + sh * sh).sqrt() * 1.25
    };
    for trace in net_bullets {
        let life = (trace.ttl / NET_BULLET_TTL).clamp(0.0, 1.0);
        let from = camera.world_to_screen((trace.from_x, trace.from_y), viewport_px);
        let mut to = camera.world_to_screen((trace.to_x, trace.to_y), viewport_px);
        let dx = to.0 - from.0;
        let dy = to.1 - from.1;
        let dist = (dx * dx + dy * dy).sqrt();
        if dist > max_trace_px {
            let s = max_trace_px / dist;
            to = (from.0 + dx * s, from.1 + dy * s);
        }
        push_line_segment(&mut masked, from, to, 4.0, [1.0, 0.97, 0.85, life * 0.75]);
        push_line_segment(&mut masked, from, to, 2.0, [1.0, 1.0, 1.0, life]);
        push_circle_fan(&mut masked, to, 4.5, [1.0, 1.0, 0.35, life * 0.85], 10);
    }

    (scene, masked)
}

fn push_rooms_debug(
    out: &mut Vec<GeoVertex>,
    rooms: &DebugRoomsView,
    camera: &TacticalCamera,
    viewport_px: (f32, f32),
) {
    for room in &rooms.rooms {
        let (x, y) = camera.world_to_screen((room.x, room.y), viewport_px);
        let (x2, y2) = camera.world_to_screen((room.x + room.w, room.y + room.h), viewport_px);
        push_rect(out, (x, y), (x2, y2), room.color);
    }

    let gap_col = [1.0, 0.75, 0.05, 0.90];
    const R: f32 = 5.0;
    for &(gx, gy) in &rooms.gaps {
        let (gx, gy) = camera.world_to_screen((gx, gy), viewport_px);
        push_diamond(out, (gx, gy), R, gap_col);
    }
}
