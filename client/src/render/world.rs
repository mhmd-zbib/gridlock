use crate::camera::TacticalCamera;
use crate::render::views::WorldView;
use game::render::quad::{QuadInstance, push_world_quad};

const WALL_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

/// Build quad instances for the static world: level floor, walls, and props.
///
/// All returned quads go into the "scene" (always-visible, dimmed outside the
/// vision cone) layer — they are world geometry, not hidden entities.
pub fn world_quads(
    view: &WorldView<'_>,
    camera: &TacticalCamera,
    viewport_px: (f32, f32),
) -> Vec<QuadInstance> {
    let mut out = Vec::new();
    let transform = camera.screen_transform(viewport_px);

    if let Some(bounds) = view.level_bounds {
        push_world_quad(
            &mut out,
            &transform,
            (bounds.x + bounds.w * 0.5, bounds.y + bounds.h * 0.5),
            (bounds.w * 0.5, bounds.h * 0.5),
            [0.07, 0.07, 0.11, 1.0],
        );
    }

    // Floors are rendered below walls and props.
    for f in &view.floors {
        if f.texture_handle.is_none() {
            push_world_quad(&mut out, &transform, f.pos, f.half_size, f.color);
        }
    }

    for w in view.walls {
        if w.breakable && !w.segments.is_empty() {
            let n = w.segments.len();
            for (i, &alive) in w.segments.iter().enumerate() {
                if !alive {
                    continue;
                }
                let (sx, sy, sw, sh) = w.segment_rect(i, n);
                push_world_quad(
                    &mut out,
                    &transform,
                    (sx + sw * 0.5, sy + sh * 0.5),
                    (sw * 0.5, sh * 0.5),
                    WALL_COLOR,
                );
            }
        } else {
            push_world_quad(
                &mut out,
                &transform,
                (w.x + w.w * 0.5, w.y + w.h * 0.5),
                (w.w * 0.5, w.h * 0.5),
                WALL_COLOR,
            );
        }
    }

    for p in &view.props {
        // Textured props are emitted as SpriteCommands in build_sprites().
        if p.texture_handle.is_none() {
            push_world_quad(&mut out, &transform, p.pos, p.half_size, p.color);
        }
    }

    out
}
