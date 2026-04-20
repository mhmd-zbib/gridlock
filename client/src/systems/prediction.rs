use std::collections::VecDeque;
use std::f32::consts::{PI, TAU};

use game::game::{Game, PLAYER_HALF};
use game::systems::movement::{
    PEEK_DISTANCE, PEEK_LERP_SPEED, apply_actor_movement, clamped_peek_distance,
};
use game::timing::FIXED_STEP;
use game::world::level::LevelBounds;
use game::world::units::px_to_tiles;
use net::{ClientPacket, MoveSpeed, SelfState, decode_rotation};

const WALK_SPEED: f32 = px_to_tiles(40.0);
const NORMAL_SPEED: f32 = px_to_tiles(85.0);
const RUN_SPEED: f32 = px_to_tiles(200.0);
const SPECTATOR_SPEED: f32 = RUN_SPEED;
const RECONCILE_SMOOTH_BLEND: f32 = 0.45;
const RECONCILE_HARD_SNAP_DIST: f32 = px_to_tiles(96.0);

pub const MAX_PENDING_LOCAL_INPUTS: usize = 256;

/// Append one predicted input while bounding queue growth under packet loss.
pub fn push_pending_input(queue: &mut VecDeque<ClientPacket>, packet: ClientPacket) {
    if queue.len() >= MAX_PENDING_LOCAL_INPUTS {
        queue.pop_front();
    }
    queue.push_back(packet);
}

/// Apply one local prediction step using the same movement rules as the server.
pub fn predict_local_player(
    game: &mut Game,
    input: &ClientPacket,
    peek_origin: &mut Option<(f32, f32)>,
    authoritative_health: Option<u8>,
) {
    let is_spectator = authoritative_health == Some(0);
    let level_bounds = game.level_bounds;
    let walls = game.walls.as_slice();

    let mut x = game.player.movement.x;
    let mut y = game.player.movement.y;
    let mut velocity_frac = game.player.movement.velocity_frac;
    let speed = speed_from_input(input);

    if is_spectator {
        apply_spectator_movement(
            &mut x,
            &mut y,
            &mut velocity_frac,
            input,
            FIXED_STEP,
            level_bounds,
        );
        *peek_origin = None;
    } else {
        let peeking = input.flags.is_peeking();
        let px = input.movement_x as f32;
        let py = input.movement_y as f32;
        let has_dir = px != 0.0 || py != 0.0;

        if peeking && peek_origin.is_none() {
            *peek_origin = Some((x, y));
        }

        if let Some(origin) = *peek_origin {
            if !peeking && has_dir {
                *peek_origin = None;
                apply_actor_movement(
                    &mut x,
                    &mut y,
                    px,
                    py,
                    speed,
                    FIXED_STEP,
                    walls,
                    PLAYER_HALF,
                    level_bounds,
                );
                velocity_frac = axis_velocity_frac(px, py);
            } else {
                let (target_x, target_y) = if peeking && has_dir {
                    let len = (px * px + py * py).sqrt();
                    let dir = (px / len, py / len);
                    let dist =
                        clamped_peek_distance(origin, dir, PEEK_DISTANCE, PLAYER_HALF, walls);
                    (origin.0 + dir.0 * dist, origin.1 + dir.1 * dist)
                } else {
                    origin
                };

                let alpha = (PEEK_LERP_SPEED * FIXED_STEP).min(1.0);
                x += (target_x - x) * alpha;
                y += (target_y - y) * alpha;
                velocity_frac = 0.0;

                if !peeking {
                    let dx = x - origin.0;
                    let dy = y - origin.1;
                    if dx * dx + dy * dy < 1.0e-6 {
                        x = origin.0;
                        y = origin.1;
                        *peek_origin = None;
                    }
                }
            }
        } else {
            apply_actor_movement(
                &mut x,
                &mut y,
                px,
                py,
                speed,
                FIXED_STEP,
                walls,
                PLAYER_HALF,
                level_bounds,
            );
            velocity_frac = axis_velocity_frac(px, py);
        }
    }

    game.player.movement.x = x;
    game.player.movement.y = y;
    game.player.movement.speed = if is_spectator { SPECTATOR_SPEED } else { speed };
    game.player.movement.velocity_frac = velocity_frac;

    let rotation = decode_rotation(input.rotation);
    game.player.sight.direction = rotation;
    game.player.aim_cone.direction = rotation;

    // Advance the aim cone spread simulation so half_angle() stays accurate in
    // multiplayer.  Without this, recoil and movement spread never change and
    // sample_direction_seeded always uses the base half-angle only.
    let stats = game.player.active_weapon_stats();
    game.player.aim_cone.set_spread_profile(
        stats.aim_base_half_angle_deg,
        stats.movement_spread_max_deg,
    );
    game.player.aim_cone.update(FIXED_STEP, velocity_frac, stats.recoil_decay_deg_per_sec);
}

/// Snap to the latest authoritative server state and replay pending local inputs.
pub fn reconcile_local_player(
    game: &mut Game,
    server_me: SelfState,
    pending_inputs: &mut VecDeque<ClientPacket>,
    peek_origin: &mut Option<(f32, f32)>,
    latest_ack_timestamp: u32,
) {
    let before_x = game.player.movement.x;
    let before_y = game.player.movement.y;
    let before_sight_dir = game.player.sight.direction;
    let before_aim_dir = game.player.aim_cone.direction;

    while pending_inputs
        .front()
        .is_some_and(|pkt| pkt.timestamp <= latest_ack_timestamp)
    {
        pending_inputs.pop_front();
    }

    game.player.movement.x = server_me.x;
    game.player.movement.y = server_me.y;
    let rotation = decode_rotation(server_me.rotation);
    game.player.sight.direction = rotation;
    game.player.aim_cone.direction = rotation;
    // Sync recoil from the server's authoritative half-angle so the local aim
    // cone state matches the yellow cone the render draws (which also reads
    // server_me.aim_cone_half_angle).  Without this, bullet sampling uses
    // a stale near-zero local recoil while the visual cone shows a wide spread.
    game.player.aim_cone.sync_from_server_half_angle(server_me.aim_cone_half_angle);

    *peek_origin = None;
    for packet in pending_inputs.iter().copied() {
        predict_local_player(game, &packet, peek_origin, Some(server_me.health));
    }

    let target_x = game.player.movement.x;
    let target_y = game.player.movement.y;
    let target_sight_dir = game.player.sight.direction;
    let target_aim_dir = game.player.aim_cone.direction;

    let dx = target_x - before_x;
    let dy = target_y - before_y;
    let hard_snap = dx * dx + dy * dy > RECONCILE_HARD_SNAP_DIST * RECONCILE_HARD_SNAP_DIST;
    if !hard_snap {
        game.player.movement.x = before_x + dx * RECONCILE_SMOOTH_BLEND;
        game.player.movement.y = before_y + dy * RECONCILE_SMOOTH_BLEND;
        game.player.sight.direction =
            lerp_angle(before_sight_dir, target_sight_dir, RECONCILE_SMOOTH_BLEND);
        game.player.aim_cone.direction =
            lerp_angle(before_aim_dir, target_aim_dir, RECONCILE_SMOOTH_BLEND);
    }
}

fn apply_spectator_movement(
    x: &mut f32,
    y: &mut f32,
    velocity_frac: &mut f32,
    input: &ClientPacket,
    dt: f32,
    level_bounds: Option<LevelBounds>,
) {
    let vx = input.movement_x as f32;
    let vy = input.movement_y as f32;
    let len = (vx * vx + vy * vy).sqrt();
    if len > 0.001 {
        *x += (vx / len) * SPECTATOR_SPEED * dt;
        *y += (vy / len) * SPECTATOR_SPEED * dt;
        *velocity_frac = 1.0;
    } else {
        *velocity_frac = 0.0;
    }
    if let Some(b) = level_bounds {
        *x = x.clamp(b.x, b.x + b.w);
        *y = y.clamp(b.y, b.y + b.h);
    }
}

fn axis_velocity_frac(x: f32, y: f32) -> f32 {
    (x * x + y * y).sqrt().min(1.0)
}

fn speed_from_input(input: &ClientPacket) -> f32 {
    match input.flags.move_speed() {
        MoveSpeed::SlowWalk => WALK_SPEED,
        MoveSpeed::Walk => NORMAL_SPEED,
        MoveSpeed::Run => RUN_SPEED,
    }
}

fn lerp_angle(from: f32, to: f32, alpha: f32) -> f32 {
    let mut delta = to - from;
    while delta > PI {
        delta -= TAU;
    }
    while delta < -PI {
        delta += TAU;
    }
    from + delta * alpha.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use net::{InputFlags, encode_rotation};

    fn packet(timestamp: u32) -> ClientPacket {
        let mut flags = InputFlags::default();
        flags.set_move_speed(MoveSpeed::Walk);
        ClientPacket {
            sequence: 0,
            timestamp,
            movement_x: 1,
            movement_y: 0,
            rotation: encode_rotation(0.0),
            flags,
        }
    }

    #[test]
    fn prediction_moves_player_immediately() {
        let mut game = Game::new();
        let start_x = game.player.movement.x;
        let mut peek_origin = None;

        predict_local_player(&mut game, &packet(100), &mut peek_origin, Some(100));

        assert!(game.player.movement.x > start_x);
    }

    #[test]
    fn reconciliation_drops_acked_and_replays_unacked() {
        let mut game = Game::new();
        let start_x = game.player.movement.x;
        let start_y = game.player.movement.y;
        let mut pending = VecDeque::from([packet(10), packet(20)]);
        let mut peek_origin = None;

        reconcile_local_player(
            &mut game,
            SelfState {
                x: start_x,
                y: start_y,
                rotation: encode_rotation(0.0),
                health: 100,
                ..SelfState::default()
            },
            &mut pending,
            &mut peek_origin,
            10,
        );

        assert_eq!(pending.len(), 1);
        assert_eq!(pending.front().map(|p| p.timestamp), Some(20));
        let expected_x = start_x + NORMAL_SPEED * FIXED_STEP;
        assert!(game.player.movement.x > start_x);
        assert!(game.player.movement.x <= expected_x + 1.0e-5);
    }

    #[test]
    fn reconciliation_smooths_small_position_corrections() {
        let mut game = Game::new();
        let server_x = game.player.movement.x;
        let server_y = game.player.movement.y;
        game.player.movement.x += px_to_tiles(48.0);
        let before_x = game.player.movement.x;
        let mut pending = VecDeque::new();
        let mut peek_origin = None;

        reconcile_local_player(
            &mut game,
            SelfState {
                x: server_x,
                y: server_y,
                rotation: encode_rotation(0.0),
                health: 100,
                ..SelfState::default()
            },
            &mut pending,
            &mut peek_origin,
            0,
        );

        assert!(game.player.movement.x < before_x);
        assert!(game.player.movement.x > server_x);
    }

    #[test]
    fn reconciliation_hard_snaps_large_position_errors() {
        let mut game = Game::new();
        let server_x = game.player.movement.x;
        let server_y = game.player.movement.y;
        game.player.movement.x += px_to_tiles(240.0);
        let mut pending = VecDeque::new();
        let mut peek_origin = None;

        reconcile_local_player(
            &mut game,
            SelfState {
                x: server_x,
                y: server_y,
                rotation: encode_rotation(0.0),
                health: 100,
                ..SelfState::default()
            },
            &mut pending,
            &mut peek_origin,
            0,
        );

        assert!((game.player.movement.x - server_x).abs() < 1.0e-6);
        assert!((game.player.movement.y - server_y).abs() < 1.0e-6);
    }
}
