use crate::camera::TacticalCamera;
use crate::render::views::EntitiesView;
use engine::render::quad::{ConeVisInstance, QuadInstance, push_quad};

/// Half-size of an enemy body quad in screen pixels.
const ENEMY_BODY_HALF_PX: f32 = 8.0;

/// A server-authoritative bullet trace, rendered as a fast fading tracer line.
pub struct NetBulletTrace {
    pub from_x: f32,
    pub from_y: f32,
    pub to_x: f32,
    pub to_y: f32,
    pub ttl: f32,
    /// True when the shooter is on the local player's team (or is the local player).
    pub friendly: bool,
}

/// Total lifetime of a network bullet tracer in seconds.
pub const NET_BULLET_TTL: f32 = 0.08;

pub struct EntityQuadLayers {
    pub scene: Vec<QuadInstance>,
    pub masked: Vec<QuadInstance>,
    /// Enemies rendered with per-pixel cone visibility (no hard stencil pop).
    pub cone_vis: Vec<ConeVisInstance>,
}

/// Build quad instances for dynamic entities: player, remote players, enemies,
/// and local bullets.
///
/// - `scene`: dimmed outside the vision cone (debug markers).
/// - `masked`: binary stencil — dim teammates-cone-only enemies, bullets.
/// - `cone_vis`: per-pixel V(P) for player-visible enemies — soft reveal.
pub fn entity_quads(
    view: &EntitiesView<'_>,
    camera: &TacticalCamera,
    viewport_px: (f32, f32),
) -> EntityQuadLayers {
    let mut scene = Vec::new();
    let mut masked = Vec::new();
    let mut cone_vis: Vec<ConeVisInstance> = Vec::new();

    // Enemies.
    for e in &view.enemies {
        let ep = camera.world_to_screen(e.pos, viewport_px);

        if e.is_visible_to_player {
            // Per-pixel cone visibility: soft angular + distance fade, no hard clip.
            cone_vis.push(ConeVisInstance {
                center: [ep.0, ep.1],
                half_size: [ENEMY_BODY_HALF_PX, ENEMY_BODY_HALF_PX],
                color: e.color,
                viewer_pos: [view.vis_viewer_pos.0, view.vis_viewer_pos.1],
                view_dir: [view.vis_view_dir.0, view.vis_view_dir.1],
                cone_a: [
                    view.vis_cos_inner,
                    view.vis_cos_outer,
                    view.vis_range_inner_px,
                    view.vis_range_outer_px,
                ],
                cone_b: [view.vis_circle_radius_px, 0.0, 0.0, 0.0],
            });
        } else {
            // Dim appearance inside stencil (teammate-cone area or outside all cones).
            push_quad(&mut masked, ep, (ENEMY_BODY_HALF_PX, ENEMY_BODY_HALF_PX), e.color);
        }

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

    EntityQuadLayers { scene, masked, cone_vis }
}
