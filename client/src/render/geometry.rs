use engine::render::geometry::{
    GeoVertex, push_circle_fan, push_cone_fan, push_diamond, push_line_segment, push_rect,
};
use game::entity::enemy::EnemyKind;
use game::game::Game;
use game::world::camera::TacticalCamera;
use game::world::rooms::LevelRooms;
use game::world::units::tiles_to_px;
use net::SelfState;
use net::decode_rotation;

use crate::render::entities::{NET_BULLET_TTL, NetBulletTrace};

/// Alpha of the black overlay applied outside the player's vision cone.
pub const OUTSIDE_CONE_DIM: f32 = 0.9;

/// Build all geometry overlays for the play state.
///
/// Returns `(scene_geo, masked_geo)`:
/// - `scene_geo` — always rendered (room debug overlay, player cone outlines).
/// - `masked_geo` — hidden outside the vision cone (enemy cones, bullet traces,
///   impact marks).
pub fn play_geometry(
    game: &Game,
    camera: &TacticalCamera,
    debug: bool,
    viewport_px: (f32, f32),
    net_bullets: &[NetBulletTrace],
    server_me: Option<&SelfState>,
) -> (Vec<GeoVertex>, Vec<GeoVertex>) {
    let mut scene = Vec::new();
    let mut masked = Vec::new();
    let walls = &game.walls;
    let player_pos = (game.player.movement.x, game.player.movement.y);
    let player_px = camera.world_to_screen(player_pos, viewport_px);

    // Debug: room / gap topology overlay.
    if debug {
        push_level_rooms_geo(&mut scene, &game.rooms, camera, viewport_px);
    }

    // Player sight circle and cone (semi-transparent fill).
    let circle = camera.world_points_to_screen(
        game.player.sight.circle_arc_pts(player_pos, walls, 64),
        viewport_px,
    );
    push_cone_fan(&mut scene, player_px, &circle, [0.3, 0.7, 1.0, 0.07]);

    let arc = camera.world_points_to_screen(
        game.player.sight.cone_arc_pts(player_pos, walls, 60),
        viewport_px,
    );
    push_cone_fan(&mut scene, player_px, &arc, [0.3, 0.7, 1.0, 0.16]);

    // Aim cone (orange, wall-clipped).
    let (aim_dir, aim_half) = if let Some(me) = server_me {
        (decode_rotation(me.rotation), me.aim_cone_half_angle)
    } else {
        (
            game.player.aim_cone.direction,
            game.player.aim_cone.half_angle(),
        )
    };
    let aim_arc = camera.world_points_to_screen(
        game.player
            .aim_cone
            .cone_arc_pts_for(player_pos, walls, 16, aim_dir, aim_half),
        viewport_px,
    );
    push_cone_fan(&mut scene, player_px, &aim_arc, [1.0, 0.6, 0.1, 0.45]);

    // Bullet impact marks.
    for impact in &game.impacts {
        let pos = camera.world_to_screen((impact.x, impact.y), viewport_px);
        push_circle_fan(
            &mut masked,
            pos,
            5.0,
            [1.0, 0.95, 0.2, 0.22 * impact.alpha()],
            18,
        );
    }

    // Enemy sight circles and cones.
    for e in &game.enemies {
        if e.kind == EnemyKind::TargetDummy {
            continue;
        }
        let visible = e.visible_to_player;
        if !visible && !debug {
            continue;
        }
        let ep = (e.movement.x, e.movement.y);
        let ep_px = camera.world_to_screen(ep, viewport_px);

        let circle_alpha = if visible { 0.05 } else { 0.03 };
        push_circle_fan(
            &mut masked,
            ep_px,
            tiles_to_px(e.sight.circle_radius),
            [1.0, 0.3, 0.3, circle_alpha],
            36,
        );

        let alpha_scale = if visible { 1.0 } else { 0.45 };
        let cone_color = if e.brain.awareness.in_combat() {
            [1.0, 0.08, 0.08, 0.35 * alpha_scale]
        } else if e.brain.awareness.is_alert() {
            [1.0, 0.50, 0.05, 0.28 * alpha_scale]
        } else {
            [1.0, 0.85, 0.20, 0.14 * alpha_scale]
        };
        let arc = camera.world_points_to_screen(e.sight.cone_arc_pts(ep, walls, 48), viewport_px);
        push_cone_fan(&mut masked, ep_px, &arc, cone_color);
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

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Render the room/gap topology computed from `LevelRooms` as a debug overlay.
fn push_level_rooms_geo(
    out: &mut Vec<GeoVertex>,
    rooms: &LevelRooms,
    camera: &TacticalCamera,
    viewport_px: (f32, f32),
) {
    for room in &rooms.rooms {
        let color = room.color;
        let (x, y) = camera.world_to_screen((room.x, room.y), viewport_px);
        let (x2, y2) = camera.world_to_screen((room.x + room.w, room.y + room.h), viewport_px);
        push_rect(out, (x, y), (x2, y2), color);
    }

    // Gap waypoints as small filled diamonds.
    let gap_col = [1.0, 0.75, 0.05, 0.90];
    const R: f32 = 5.0;
    for &(gx, gy) in &rooms.gaps {
        let (gx, gy) = camera.world_to_screen((gx, gy), viewport_px);
        push_diamond(out, (gx, gy), R, gap_col);
    }
}
