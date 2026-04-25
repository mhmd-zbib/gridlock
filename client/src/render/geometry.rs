use crate::camera::TacticalCamera;
use crate::render::entities::{NET_BULLET_TTL, NetBulletTrace};
use crate::render::sight_geometry::{aim_cone_arc_pts, cone_arc_pts_raw};
use crate::render::views::{DebugRoomsView, GeometryView, PlayerCircleView, SoundFieldView};
use engine::render::geometry::{
    GeoVertex, push_circle_fan, push_cone_fan, push_diamond, push_line_segment, push_rect,
};
use game::world::units::px_to_tiles;
use game::world::wall::Wall;

const COLLIDER_OUTLINE_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 0.90];
const COLLIDER_OUTLINE_WIDTH_PX: f32 = 1.0;
// Half a pixel in world units — shifts the line fully inside the wall boundary.
const COLLIDER_INSET_WORLD: f32 = px_to_tiles(0.5);

type EdgeSegment = ((f32, f32), (f32, f32));

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

    // Sound field: boundary ring + pulse rings (always in scene layer).
    if let Some(sf) = &view.sound_field {
        push_sound_field(&mut scene, camera.world_to_screen(sf.pos, viewport_px), sf);
    }

    // Aim cone (orange, wall-clipped) — hidden while spectating.
    if !view.is_spectating {
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
        push_cone_fan(&mut scene, player_px, &aim_arc, [1.0, 0.6, 0.1, 0.20]);
    }

    // Wall outlines go into masked: the GPU stencil (vision cone) hides them
    // outside the FOV. The line is outset by half a pixel so it sits in the
    // stencil=1 zone (between player and wall face), not inside the wall.
    push_collider_outlines(&mut masked, view, camera, viewport_px);

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

    // Players rendered as filled circles in the masked layer.
    for p in &view.player_circles {
        push_player_circle(&mut masked, p, camera, viewport_px);
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
        // Friendly bullets go in scene (always visible); enemy bullets stay masked.
        let buf = if trace.friendly {
            &mut scene
        } else {
            &mut masked
        };
        push_line_segment(buf, from, to, 4.0, [1.0, 0.97, 0.85, life * 0.75]);
        push_line_segment(buf, from, to, 2.0, [1.0, 1.0, 1.0, life]);
        push_circle_fan(buf, to, 4.5, [1.0, 1.0, 0.35, life * 0.85], 10);
    }

    (scene, masked)
}

fn push_collider_outlines(
    out: &mut Vec<GeoVertex>,
    view: &GeometryView<'_>,
    camera: &TacticalCamera,
    viewport_px: (f32, f32),
) {
    for wall in view.walls {
        for (x, y, w, h) in wall_outline_rects(wall) {
            // Outset by half a pixel so the 1px line sits just outside the wall
            // face, fully within stencil=1 (vision cone zone), never inside the wall.
            let i = COLLIDER_INSET_WORLD;
            for (from, to) in rect_outline_edges((x - i, y - i, w + 2.0 * i, h + 2.0 * i)) {
                let from_px = camera.world_to_screen(from, viewport_px);
                let to_px = camera.world_to_screen(to, viewport_px);
                push_line_segment(
                    out,
                    from_px,
                    to_px,
                    COLLIDER_OUTLINE_WIDTH_PX,
                    COLLIDER_OUTLINE_COLOR,
                );
            }
        }
    }
}

fn wall_outline_rects(wall: &Wall) -> Vec<(f32, f32, f32, f32)> {
    if wall.breakable && !wall.segments.is_empty() {
        let n = wall.segments.len();
        return wall
            .segments
            .iter()
            .enumerate()
            .filter_map(|(idx, alive)| {
                if *alive {
                    Some(wall.segment_rect(idx, n))
                } else {
                    None
                }
            })
            .collect();
    }

    vec![(wall.x, wall.y, wall.w, wall.h)]
}

fn rect_outline_edges(rect: (f32, f32, f32, f32)) -> [EdgeSegment; 4] {
    let (x, y, w, h) = rect;
    [
        ((x, y), (x + w, y)),
        ((x + w, y), (x + w, y + h)),
        ((x + w, y + h), (x, y + h)),
        ((x, y + h), (x, y)),
    ]
}

fn push_circle_outline(
    out: &mut Vec<GeoVertex>,
    center: (f32, f32),
    radius: f32,
    width: f32,
    color: [f32; 4],
    n: usize,
) {
    use std::f32::consts::TAU;
    for i in 0..n {
        let a0 = (i as f32 / n as f32) * TAU;
        let a1 = ((i + 1) as f32 / n as f32) * TAU;
        let p0 = (center.0 + radius * a0.cos(), center.1 + radius * a0.sin());
        let p1 = (center.0 + radius * a1.cos(), center.1 + radius * a1.sin());
        push_line_segment(out, p0, p1, width, color);
    }
}

fn push_sound_field(out: &mut Vec<GeoVertex>, center: (f32, f32), sf: &SoundFieldView) {
    // Boundary ring: dim at rest, brighter when moving.
    let edge_alpha = 0.12 + 0.40 * sf.speed_frac;
    push_circle_outline(out, center, sf.smoothed_radius_px, 1.5, [0.35, 0.85, 1.0, edge_alpha], 48);

    // Expanding pulse rings — amplitude drives opacity.
    for ring in &sf.rings {
        if ring.radius_px > 0.5 {
            push_circle_outline(out, center, ring.radius_px, 2.0, [0.55, 0.92, 1.0, ring.alpha * 0.65], 36);
        }
    }
}

fn push_player_circle(
    out: &mut Vec<GeoVertex>,
    p: &PlayerCircleView,
    camera: &TacticalCamera,
    viewport_px: (f32, f32),
) {
    let center = camera.world_to_screen(p.pos, viewport_px);
    push_circle_fan(out, center, 8.0, p.color, 24);
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
