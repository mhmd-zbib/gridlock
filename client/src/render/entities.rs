use crate::camera::TacticalCamera;
use crate::render::views::EntitiesView;
use game::render::quad::{QuadInstance, push_quad};

/// Half-size of an enemy body quad in screen pixels.
const ENEMY_BODY_HALF_PX: f32 = 8.0;

/// A server-authoritative bullet trace, rendered as a fast fading tracer line.
pub struct NetBulletTrace {
    pub from_x: f32,
    pub from_y: f32,
    pub to_x: f32,
    pub to_y: f32,
    pub ttl: f32,
}

/// Total lifetime of a network bullet tracer in seconds.
pub const NET_BULLET_TTL: f32 = 0.08;

/// Build quad instances for dynamic entities: player, remote players, enemies,
/// and local bullets.
///
/// Returns `(scene_quads, masked_quads)`.
/// - `scene_quads` are dimmed outside the vision cone.
/// - `masked_quads` are completely hidden outside the cone.
pub fn entity_quads(
    view: &EntitiesView<'_>,
    camera: &TacticalCamera,
    viewport_px: (f32, f32),
) -> (Vec<QuadInstance>, Vec<QuadInstance>) {
    let mut scene = Vec::new();
    let mut masked = Vec::new();

    // Local player — always visible, masked by vision cone.
    let player_screen = camera.world_to_screen(view.player_pos, viewport_px);
    push_quad(
        &mut masked,
        player_screen,
        (10.0, 10.0),
        [1.0, 1.0, 1.0, 1.0],
    );

    // Remote players (server-authoritative positions).
    for remote in view.remote_players {
        let p = camera.world_to_screen((remote.x, remote.y), viewport_px);
        push_quad(&mut masked, p, (9.0, 9.0), [0.95, 0.25, 0.25, 1.0]);
    }

    // Enemies.
    for e in &view.enemies {
        let ep = camera.world_to_screen(e.pos, viewport_px);
        push_quad(
            &mut masked,
            ep,
            (ENEMY_BODY_HALF_PX, ENEMY_BODY_HALF_PX),
            e.color,
        );

        if let Some(dbg) = &e.debug {
            let anchor = camera.world_to_screen(dbg.spawn_anchor, viewport_px);
            push_quad(&mut scene, anchor, (4.0, 4.0), [0.3, 0.5, 1.0, 0.8]);

            if let Some(lk) = dbg.last_known_pos {
                let lk = camera.world_to_screen(lk, viewport_px);
                push_quad(&mut scene, lk, (5.0, 5.0), [1.0, 0.3, 1.0, 0.85]);
            }

            if let Some(mv) = dbg.last_move_target {
                let mv = camera.world_to_screen(mv, viewport_px);
                push_quad(&mut scene, mv, (4.0, 4.0), [0.2, 1.0, 0.4, 0.85]);
            }

            for gap in &dbg.gap_waypoints {
                let gap = camera.world_to_screen(*gap, viewport_px);
                push_quad(&mut scene, gap, (4.0, 4.0), [0.0, 0.9, 0.9, 0.75]);
            }
        }
    }

    // Local bullets.
    for &pos in &view.bullet_positions {
        let bullet = camera.world_to_screen(pos, viewport_px);
        push_quad(&mut masked, bullet, (3.0, 3.0), [1.0, 1.0, 0.0, 1.0]);
    }

    (scene, masked)
}
