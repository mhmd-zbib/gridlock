use crate::camera::TacticalCamera;
use crate::render::views::WorldView;
use engine::render::quad::{QuadInstance, push_world_quad};

const WALL_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

pub struct WorldQuadLayers {
    /// Non-wall world quads: bounds + untextured floors/props.
    pub scene: Vec<QuadInstance>,
    /// Wall quads rendered above world sprites.
    pub walls: Vec<QuadInstance>,
}

/// Build quad instances for the static world: level floor, walls, and props.
///
/// Returns split world layers so callers can draw walls above floor/prop
/// sprites while keeping all world layers in the always-visible scene path.
pub fn world_quads(
    view: &WorldView<'_>,
    camera: &TacticalCamera,
    viewport_px: (f32, f32),
) -> WorldQuadLayers {
    let mut scene = Vec::new();
    let mut walls = Vec::new();
    let transform = camera.screen_transform(viewport_px);

    if let Some(bounds) = view.level_bounds {
        push_world_quad(
            &mut scene,
            &transform,
            (bounds.x + bounds.w * 0.5, bounds.y + bounds.h * 0.5),
            (bounds.w * 0.5, bounds.h * 0.5),
            [0.07, 0.07, 0.11, 1.0],
        );
    }

    // Floors are rendered below props and walls.
    for f in &view.floors {
        if f.texture_handle.is_none() {
            push_world_quad(&mut scene, &transform, f.pos, f.half_size, f.color);
        }
    }

    // Untextured props sit above floors, below walls.
    for p in &view.props {
        // Textured props are emitted as SpriteCommands in build_sprites().
        if p.texture_handle.is_none() {
            push_world_quad(&mut scene, &transform, p.pos, p.half_size, p.color);
        }
    }

    // Walls are emitted into a dedicated top world layer.
    // Prop-promoted walls are invisible — the prop sprite owns the visual.
    for w in view.walls.iter().filter(|w| !w.is_prop) {
        if w.breakable && !w.segments.is_empty() {
            let n = w.segments.len();
            for (i, &alive) in w.segments.iter().enumerate() {
                if !alive {
                    continue;
                }
                let (sx, sy, sw, sh) = w.segment_rect(i, n);
                push_world_quad(
                    &mut walls,
                    &transform,
                    (sx + sw * 0.5, sy + sh * 0.5),
                    (sw * 0.5, sh * 0.5),
                    WALL_COLOR,
                );
            }
        } else {
            push_world_quad(
                &mut walls,
                &transform,
                (w.x + w.w * 0.5, w.y + w.h * 0.5),
                (w.w * 0.5, w.h * 0.5),
                WALL_COLOR,
            );
        }
    }

    WorldQuadLayers { scene, walls }
}
