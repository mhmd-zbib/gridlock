use engine::render::quad::QuadInstance;
use game::game::Game;
use game::world::camera::TacticalCamera;
use game::world::prop;
use game::world::units::tiles_to_px;

/// Build quad instances for the static world: level floor, walls, and props.
///
/// All returned quads go into the "scene" (always-visible, dimmed outside the
/// vision cone) layer — they are world geometry, not hidden entities.
pub fn world_quads(
    game: &Game,
    camera: &TacticalCamera,
    viewport_px: (f32, f32),
) -> Vec<QuadInstance> {
    let mut out = Vec::new();

    // Level floor (fills the authoritative map bounds).
    if let Some(bounds) = game.level_bounds {
        let center = camera.world_to_screen(
            (bounds.x + bounds.w * 0.5, bounds.y + bounds.h * 0.5),
            viewport_px,
        );
        out.push(QuadInstance {
            center: [center.0, center.1],
            half_size: [tiles_to_px(bounds.w * 0.5), tiles_to_px(bounds.h * 0.5)],
            color: [0.07, 0.07, 0.11, 1.0],
        });
    }

    // Walls — breakable walls are rendered segment by segment.
    for w in &game.walls {
        if w.breakable && !w.segments.is_empty() {
            let n = w.segments.len();
            for (i, &alive) in w.segments.iter().enumerate() {
                if !alive {
                    continue;
                }
                let (sx, sy, sw, sh) = w.segment_rect(i, n);
                let center = camera.world_to_screen((sx + sw * 0.5, sy + sh * 0.5), viewport_px);
                out.push(QuadInstance {
                    center: [center.0, center.1],
                    half_size: [tiles_to_px(sw * 0.5), tiles_to_px(sh * 0.5)],
                    color: [0.2, 0.8, 0.95, 1.0],
                });
            }
        } else {
            let center = camera.world_to_screen((w.x + w.w * 0.5, w.y + w.h * 0.5), viewport_px);
            out.push(QuadInstance {
                center: [center.0, center.1],
                half_size: [tiles_to_px(w.w * 0.5), tiles_to_px(w.h * 0.5)],
                color: if w.breakable {
                    [0.2, 0.8, 0.95, 1.0]
                } else {
                    [0.45, 0.4, 0.35, 1.0]
                },
            });
        }
    }

    // Props (decorative + collidable).
    for p in &game.props {
        let center = camera.world_to_screen((p.x, p.y), viewport_px);
        out.push(QuadInstance {
            center: [center.0, center.1],
            half_size: [tiles_to_px(p.width * 0.5), tiles_to_px(p.height * 0.5)],
            color: prop::asset_color(&p.id, p.is_collider, 1.0),
        });
    }

    out
}
