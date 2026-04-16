use engine::render::quad::QuadInstance;
use game::entity::enemy::EnemyKind;
use game::game::Game;
use game::world::camera::TacticalCamera;
use net::PlayerState;

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
/// and local bullets.  Also emits debug quads (anchors, waypoints, …) when
/// `debug` is enabled.
///
/// Returns `(scene_quads, masked_quads)`.
/// - `scene_quads` are dimmed outside the vision cone.
/// - `masked_quads` are completely hidden outside the cone.
pub fn entity_quads(
    game: &Game,
    camera: &TacticalCamera,
    debug: bool,
    viewport_px: (f32, f32),
    remote_players: &[PlayerState],
) -> (Vec<QuadInstance>, Vec<QuadInstance>) {
    let mut scene = Vec::new();
    let mut masked = Vec::new();

    // Local player — always visible, masked by vision cone.
    let player_screen = camera.world_to_screen(
        (game.player.movement.x, game.player.movement.y),
        viewport_px,
    );
    masked.push(QuadInstance {
        center: [player_screen.0, player_screen.1],
        half_size: [10.0, 10.0],
        color: [1.0, 1.0, 1.0, 1.0],
    });

    // Remote players (server-authoritative positions).
    for remote in remote_players {
        let p = camera.world_to_screen((remote.x, remote.y), viewport_px);
        masked.push(QuadInstance {
            center: [p.0, p.1],
            half_size: [9.0, 9.0],
            color: [0.95, 0.25, 0.25, 1.0],
        });
    }

    // Enemies.
    for e in &game.enemies {
        let color = enemy_body_color(e.kind, e.visible_to_player);
        let ep = camera.world_to_screen((e.movement.x, e.movement.y), viewport_px);
        masked.push(QuadInstance {
            center: [ep.0, ep.1],
            half_size: [ENEMY_BODY_HALF_PX, ENEMY_BODY_HALF_PX],
            color,
        });

        if debug && e.kind == EnemyKind::Shooter {
            let pos = (e.movement.x, e.movement.y);

            // Spawn anchor — blue dot.
            let anchor = camera.world_to_screen(e.brain.spawn_anchor(), viewport_px);
            scene.push(QuadInstance {
                center: [anchor.0, anchor.1],
                half_size: [4.0, 4.0],
                color: [0.3, 0.5, 1.0, 0.8],
            });

            // Last known player position — magenta dot.
            if let Some(lk) = e.brain.awareness.last_known_pos() {
                let lk = camera.world_to_screen(lk, viewport_px);
                scene.push(QuadInstance {
                    center: [lk.0, lk.1],
                    half_size: [5.0, 5.0],
                    color: [1.0, 0.3, 1.0, 0.85],
                });
            }

            // Current move target — green dot.
            if let Some(mv) = e.brain.last_move_target {
                let mv = camera.world_to_screen(mv, viewport_px);
                scene.push(QuadInstance {
                    center: [mv.0, mv.1],
                    half_size: [4.0, 4.0],
                    color: [0.2, 1.0, 0.4, 0.85],
                });
            }

            // Gap waypoints — cyan dots.
            for gap in e.brain.debug_gaps(pos, &game.walls) {
                let gap = camera.world_to_screen(gap, viewport_px);
                scene.push(QuadInstance {
                    center: [gap.0, gap.1],
                    half_size: [4.0, 4.0],
                    color: [0.0, 0.9, 0.9, 0.75],
                });
            }
        }
    }

    // Local bullets.
    for b in &game.bullets {
        let bullet = camera.world_to_screen((b.x, b.y), viewport_px);
        masked.push(QuadInstance {
            center: [bullet.0, bullet.1],
            half_size: [3.0, 3.0],
            color: [1.0, 1.0, 0.0, 1.0],
        });
    }

    (scene, masked)
}

fn enemy_body_color(kind: EnemyKind, visible: bool) -> [f32; 4] {
    match (kind, visible) {
        (EnemyKind::Shooter, true) => [1.0, 0.2, 0.2, 1.0],
        (EnemyKind::Shooter, false) => [0.6, 0.15, 0.15, 0.55],
        (EnemyKind::TargetDummy, true) => [1.0, 0.85, 0.2, 1.0],
        (EnemyKind::TargetDummy, false) => [0.6, 0.5, 0.12, 0.55],
    }
}
