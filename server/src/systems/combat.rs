use std::net::SocketAddr;

use crate::session::ServerState;
use game::world::ray::cast_ray;
use game::world::units::px_to_tiles;
use game::world::wall::Wall;
use net::decode_rotation;
use net::proto::server::BulletEvent;
use net::MoveSpeed;

const PLAYER_HALF: f32 = px_to_tiles(10.0);
const BULLET_MAX_RANGE: f32 = px_to_tiles(1500.0);
/// Small epsilon pushed along the ray direction before testing wall damage so
/// the impact point is safely inside the wall AABB.
const WALL_HIT_EPS: f32 = px_to_tiles(0.5);

/// Process one combat tick for all sessions that are currently shooting.
///
/// For each shooter whose weapon fires successfully this tick a ray-cast
/// resolves the trajectory:
/// 1. Find the nearest wall.
/// 2. Find the nearest enemy player (circle intersection).
/// 3. Apply damage to the first thing hit.
/// 4. Emit a `BulletEvent` for every shot so clients can render tracers.
pub fn step_combat(st: &mut ServerState, walls: &mut Vec<Wall>, dt: f32) -> Vec<BulletEvent> {
    let shooter_addrs: Vec<SocketAddr> = st.sessions.keys().copied().collect();
    let mut bullets = Vec::new();

    for shooter_addr in shooter_addrs {
        let Some((shooter_id, origin, dir, damage)) = tick_shooter(st, shooter_addr, dt) else {
            continue;
        };

        let wall_dist = cast_ray(origin, dir, BULLET_MAX_RANGE, walls);
        let mut impact_dist = wall_dist;
        let mut hit_target_addr: Option<SocketAddr> = None;

        for (&target_addr, target) in &st.sessions {
            if target_addr == shooter_addr || target.health == 0 {
                continue;
            }
            if let Some(hit_dist) =
                ray_circle_hit_distance(origin, dir, (target.x, target.y), PLAYER_HALF, impact_dist)
            {
                impact_dist = hit_dist;
                hit_target_addr = Some(target_addr);
            }
        }

        let impact_x = origin.0 + dir.0 * impact_dist;
        let impact_y = origin.1 + dir.1 * impact_dist;

        let mut hit_player_id = 0u16;
        if let Some(target_addr) = hit_target_addr {
            if let Some(target) = st.sessions.get_mut(&target_addr) {
                let clamped = damage.min(u16::MAX as u32) as u16;
                target.health = target.health.saturating_sub(clamped);
                hit_player_id = target.player_id;
            }
        } else if wall_dist < BULLET_MAX_RANGE {
            apply_wall_hit(
                walls,
                (
                    impact_x + dir.0 * WALL_HIT_EPS,
                    impact_y + dir.1 * WALL_HIT_EPS,
                ),
                damage,
            );
        }

        bullets.push(BulletEvent {
            shooter_id,
            from_x: origin.0,
            from_y: origin.1,
            to_x: impact_x,
            to_y: impact_y,
            hit_player_id,
        });
    }

    bullets
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Tick one shooter's weapon and, if a shot fires, return:
/// `(shooter_id, origin, normalised_direction, damage)`.
///
/// Returns `None` when the player is dead, has no latest input, or the weapon
/// did not fire this tick (rate-limited or reloading).
fn tick_shooter(
    st: &mut ServerState,
    shooter_addr: SocketAddr,
    dt: f32,
) -> Option<(u16, (f32, f32), (f32, f32), u32)> {
    let shooter = st.sessions.get_mut(&shooter_addr)?;
    if shooter.health == 0 {
        return None;
    }

    shooter.weapon.tick(dt);

    let latest_input = shooter.latest_input?;
    if latest_input.flags.is_reloading() {
        let _ = shooter.weapon.try_start_reload();
    }
    if latest_input.flags.is_shooting() && shooter.weapon.ammo_in_mag() == 0 {
        let _ = shooter.weapon.try_start_reload();
    }
    shooter
        .movement_state
        .set_reloading(shooter.weapon.is_reloading());

    let weapon_stats = shooter.weapon.stats();
    shooter.aim_cone.set_spread_profile(
        weapon_stats.aim_base_half_angle_deg,
        weapon_stats.movement_spread_max_deg,
    );
    let velocity_frac = velocity_frac_from_input(&latest_input);
    shooter
        .aim_cone
        .update(dt, velocity_frac, weapon_stats.recoil_decay_deg_per_sec);
    shooter.aim_cone.direction = decode_rotation(shooter.rotation);

    if !latest_input.flags.is_shooting() {
        return None;
    }

    if !shooter.weapon.try_fire_with_stats(weapon_stats) {
        return None;
    }

    let dir = shooter.aim_cone.sample_direction();
    shooter
        .aim_cone
        .on_shot(weapon_stats.recoil_per_shot_deg, weapon_stats.recoil_max_deg);
    Some((
        shooter.player_id,
        (shooter.x, shooter.y),
        dir,
        weapon_stats.bullet_damage,
    ))
}

/// Analytic ray–sphere intersection. Returns the distance along `dir` to the
/// nearest entry point of the circle, or `None` if the ray misses or the
/// intersection is beyond `max_dist`.
fn ray_circle_hit_distance(
    origin: (f32, f32),
    dir: (f32, f32),
    center: (f32, f32),
    radius: f32,
    max_dist: f32,
) -> Option<f32> {
    let oc_x = center.0 - origin.0;
    let oc_y = center.1 - origin.1;
    let proj = oc_x * dir.0 + oc_y * dir.1;
    if proj < 0.0 || proj > max_dist {
        return None;
    }
    let oc_sq = oc_x * oc_x + oc_y * oc_y;
    let closest_sq = oc_sq - proj * proj;
    let radius_sq = radius * radius;
    if closest_sq > radius_sq {
        return None;
    }
    let offset = (radius_sq - closest_sq).sqrt();
    let mut t = proj - offset;
    if t < 0.0 {
        t = proj + offset;
    }
    (t >= 0.0 && t <= max_dist).then_some(t)
}

/// Map the client's reported move speed to a 0–1 velocity fraction.
/// Mirrors the ratio used by the client: speed / RUN_SPEED.
fn velocity_frac_from_input(input: &net::ClientPacket) -> f32 {
    let is_moving = input.movement_x != 0 || input.movement_y != 0;
    if !is_moving {
        return 0.0;
    }
    match input.flags.move_speed() {
        MoveSpeed::SlowWalk => 40.0 / 200.0,
        MoveSpeed::Walk => 85.0 / 200.0,
        MoveSpeed::Run => 1.0,
    }
}

/// Apply bullet damage to the first wall that contains the hit point.
/// Removes the wall from the vec if it is fully destroyed.
fn apply_wall_hit(walls: &mut Vec<Wall>, hit: (f32, f32), damage: u32) {
    let Some(hit_idx) = walls.iter().position(|w| w.contains(hit.0, hit.1)) else {
        return;
    };
    let destroyed = {
        let wall = &mut walls[hit_idx];
        wall.take_damage_at(hit.0, hit.1, damage)
    };
    if destroyed {
        walls.remove(hit_idx);
    }
}
