use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use game::entity::weapon::{WeaponState, all_weapon_ids};
use game::game::Game;
use game::world::level::{LevelBounds, LevelData};
use game::world::ray::cast_ray;
use game::world::sight::Sight;
use game::world::units::px_to_tiles;
use game::world::wall::{self, Wall};
use net::proto::server::{
    BulletEvent, MatchState, MovementState, PlayerState, SelfState, ServerPacket,
};
use net::{
    AnyPacket, ClientPacket, ConnectAck, ConnectResult, LobbyCommandKind, LobbyState, MoveSpeed,
    NetSocket, PROTOCOL_VERSION, decode_rotation, encode, encode_rotation,
};
use tokio::time::{Instant as TokioInstant, sleep_until};

const TICK_RATE: u64 = 60;
const TICK_DURATION: Duration = Duration::from_nanos(1_000_000_000 / TICK_RATE);
const UDP_PORT: u16 = 7777;
const MAX_PLAYERS: usize = 16;
const PLAYER_HALF: f32 = px_to_tiles(10.0);
const WALK_SPEED: f32 = px_to_tiles(40.0);
const NORMAL_SPEED: f32 = px_to_tiles(85.0);
const RUN_SPEED: f32 = px_to_tiles(200.0);
const BULLET_MAX_RANGE: f32 = px_to_tiles(1500.0);
const WALL_HIT_EPS: f32 = px_to_tiles(0.5);
const MAX_HEALTH: u16 = 100;

// ── Session ───────────────────────────────────────────────────────────────────

struct Session {
    player_id: u16,
    name: String,
    x: f32,
    y: f32,
    health: u16,
    rotation: u16,
    weapon: WeaponState,
    aim_cone_half_angle: f32,
    movement_state: MovementState,
    /// Team selection (`0` = none, `1` = team 1, `2` = team 2).
    team: u8,
    /// Latest input received from this client (updated every tick).
    latest_input: Option<ClientPacket>,
}

struct ServerState {
    sessions: HashMap<SocketAddr, Session>,
    next_player_id: u16,
    game_started: bool,
    spawn: (f32, f32),
}

impl ServerState {
    fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            next_player_id: 1,
            game_started: false,
            spawn: (px_to_tiles(400.0), px_to_tiles(300.0)),
        }
    }
}

type Shared = Arc<Mutex<ServerState>>;

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let socket: Arc<NetSocket> = Arc::new(
        NetSocket::bind(format!("0.0.0.0:{UDP_PORT}").parse().unwrap())
            .await
            .expect("failed to bind UDP socket"),
    );

    let state: Shared = Arc::new(Mutex::new(ServerState::new()));

    let recv_socket = Arc::clone(&socket);
    let recv_state = Arc::clone(&state);
    tokio::spawn(async move {
        recv_loop(recv_socket, recv_state).await;
    });

    let mut game = Game::new();
    // Load the first available level so the server's player spawn, walls, and
    // map bounds match what the client has loaded.  Without this, bullet traces
    // are computed at the server's default spawn (≈ 0,0 in px) while the
    // client's camera is centred on the level spawn — putting every trace
    // hundreds of pixels off-screen.
    if let Some(level) = load_first_level() {
        let level_w = level.map_bounds.map_or(0.0, |b| b.x + b.w);
        let level_h = level.map_bounds.map_or(0.0, |b| b.y + b.h);
        game.load_level(&level, level_w, level_h);
        {
            let mut st = state.lock().unwrap();
            st.spawn = (game.player.movement.x, game.player.movement.y);
        }
        println!(
            "[server] loaded level '{}' (spawn={:.2},{:.2})",
            level.id,
            level.player_spawn.map_or(0.0, |s| s.x),
            level.player_spawn.map_or(0.0, |s| s.y)
        );
    } else {
        println!("[server] no level found, using default spawn");
    }
    let mut tick: u32 = 0;
    let mut next_tick = TokioInstant::now();

    loop {
        sleep_until(next_tick).await;

        // Advance all connected player replicas.
        let (game_started, addrs): (bool, Vec<SocketAddr>) = {
            let mut st = state.lock().unwrap();
            for session in st.sessions.values_mut() {
                if session.health == 0 {
                    continue;
                }
                if let Some(input) = session.latest_input {
                    apply_session_input(
                        session,
                        &input,
                        TICK_DURATION.as_secs_f32(),
                        &game.walls,
                        game.level_bounds,
                    );
                }
            }
            let addrs = st.sessions.keys().copied().collect();
            (st.game_started, addrs)
        };

        if !game_started {
            tick = tick.wrapping_add(1);
            next_tick += TICK_DURATION;
            if TokioInstant::now() > next_tick {
                next_tick = TokioInstant::now() + TICK_DURATION;
            }
            continue;
        }

        let bullets = {
            let mut st = state.lock().unwrap();
            step_combat(&mut st, &mut game.walls, TICK_DURATION.as_secs_f32())
        };

        tick = tick.wrapping_add(1);
        next_tick += TICK_DURATION;

        if !addrs.is_empty() {
            let payloads: Vec<(SocketAddr, Vec<u8>)> = {
                let st = state.lock().unwrap();
                st.sessions
                    .iter()
                    .map(|(&addr, session)| {
                        let snapshot = build_snapshot_for_player(
                            tick,
                            &bullets,
                            &game.walls,
                            &st,
                            addr,
                            session,
                        );
                        (addr, encode(&AnyPacket::ServerSnapshot(snapshot)))
                    })
                    .collect()
            };
            for (addr, payload) in payloads {
                let _ = socket.send_raw(&payload, addr).await;
            }
        }

        if TokioInstant::now() > next_tick {
            next_tick = TokioInstant::now() + TICK_DURATION;
        }
    }
}

// ── Receive loop ──────────────────────────────────────────────────────────────

async fn recv_loop(socket: Arc<NetSocket>, state: Shared) {
    loop {
        match socket.recv().await {
            Ok((packet, addr)) => handle_packet(packet, addr, &socket, &state).await,
            Err(_) => {}
        }
    }
}

async fn handle_packet(packet: AnyPacket, addr: SocketAddr, socket: &NetSocket, state: &Shared) {
    match packet {
        AnyPacket::ConnectRequest(req) => {
            enum Decision {
                ReAck(u16),
                NewPlayer(u16),
                Full,
                MatchStarted,
                VersionMismatch,
            }

            let decision = {
                let mut st = state.lock().unwrap();
                if let Some(s) = st.sessions.get(&addr) {
                    Decision::ReAck(s.player_id)
                } else if req.version != PROTOCOL_VERSION {
                    Decision::VersionMismatch
                } else if st.game_started {
                    Decision::MatchStarted
                } else if st.sessions.len() >= MAX_PLAYERS {
                    Decision::Full
                } else {
                    let pid = st.next_player_id;
                    st.next_player_id = pid.wrapping_add(1).max(1);
                    let spawn = st.spawn;
                    let offset = spawn_offset(pid);
                    let weapon = default_weapon_state();
                    let aim_cone_half_angle = weapon.stats().aim_base_half_angle_deg.to_radians();
                    st.sessions.insert(
                        addr,
                        Session {
                            player_id: pid,
                            name: req.name_str().to_string(),
                            x: spawn.0 + offset.0,
                            y: spawn.1 + offset.1,
                            health: MAX_HEALTH,
                            rotation: encode_rotation(0.0),
                            weapon,
                            aim_cone_half_angle,
                            movement_state: MovementState::default(),
                            team: 0,
                            latest_input: None,
                        },
                    );
                    Decision::NewPlayer(pid)
                }
            };

            match decision {
                Decision::Full => {
                    let _ = socket
                        .send_to(
                            &AnyPacket::ConnectAck(ConnectAck {
                                result: ConnectResult::ServerFull,
                                player_id: 0,
                                tick_rate: TICK_RATE as u8,
                                server_time: server_time_ms(),
                            }),
                            addr,
                        )
                        .await;
                }
                Decision::MatchStarted => {
                    let _ = socket
                        .send_to(
                            &AnyPacket::ConnectAck(ConnectAck {
                                result: ConnectResult::MatchStarted,
                                player_id: 0,
                                tick_rate: TICK_RATE as u8,
                                server_time: server_time_ms(),
                            }),
                            addr,
                        )
                        .await;
                }
                Decision::VersionMismatch => {
                    let _ = socket
                        .send_to(
                            &AnyPacket::ConnectAck(ConnectAck {
                                result: ConnectResult::VersionMismatch,
                                player_id: 0,
                                tick_rate: TICK_RATE as u8,
                                server_time: server_time_ms(),
                            }),
                            addr,
                        )
                        .await;
                }
                Decision::ReAck(pid) | Decision::NewPlayer(pid) => {
                    let _ = socket
                        .accept(&req, addr, pid, TICK_RATE as u8, server_time_ms())
                        .await;
                    broadcast_lobby_state(socket, state).await;
                }
            }
        }

        AnyPacket::Disconnect => {
            state.lock().unwrap().sessions.remove(&addr);
            broadcast_lobby_state(socket, state).await;
        }

        AnyPacket::ClientInput(input) => {
            if let Some(session) = state.lock().unwrap().sessions.get_mut(&addr) {
                session.latest_input = Some(input);
            }
        }

        AnyPacket::LobbyCommand(cmd) => {
            let mut should_broadcast = false;
            {
                let mut st = state.lock().unwrap();
                if !st.game_started {
                    match cmd.kind {
                        LobbyCommandKind::SelectTeam => {
                            if let Some(session) = st.sessions.get_mut(&addr) {
                                session.team = if cmd.team == 2 { 2 } else { 1 };
                                should_broadcast = true;
                            }
                        }
                        LobbyCommandKind::StartGame => {
                            if st.sessions.contains_key(&addr) {
                                st.game_started = true;
                                should_broadcast = true;
                            }
                        }
                    }
                }
            }
            if should_broadcast {
                broadcast_lobby_state(socket, state).await;
            }
        }

        // Ping is a low-level protocol concern handled by NetSocket; ignore here
        AnyPacket::Ping(ts) => {
            let _ = socket.send_to(&AnyPacket::Pong(ts), addr).await;
        }

        _ => {}
    }
}

fn lobby_state_for(st: &ServerState, addr: SocketAddr) -> LobbyState {
    let mut team1 = 0u8;
    let mut team2 = 0u8;
    for session in st.sessions.values() {
        if session.team == 1 {
            team1 = team1.saturating_add(1);
        } else if session.team == 2 {
            team2 = team2.saturating_add(1);
        }
    }

    LobbyState {
        game_started: st.game_started,
        your_team: st.sessions.get(&addr).map(|s| s.team).unwrap_or(0),
        team1_count: team1,
        team2_count: team2,
    }
}

async fn broadcast_lobby_state(socket: &NetSocket, state: &Shared) {
    let per_client_packets: Vec<(SocketAddr, Vec<u8>)> = {
        let st = state.lock().unwrap();
        st.sessions
            .keys()
            .copied()
            .map(|addr| {
                let payload = encode(&AnyPacket::LobbyState(lobby_state_for(&st, addr)));
                (addr, payload)
            })
            .collect()
    };

    for (addr, payload) in per_client_packets {
        let _ = socket.send_raw(&payload, addr).await;
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn build_snapshot_for_player(
    tick: u32,
    bullets: &[BulletEvent],
    walls: &[Wall],
    st: &ServerState,
    recipient_addr: SocketAddr,
    recipient_session: &Session,
) -> ServerPacket {
    let me_x = recipient_session.x;
    let me_y = recipient_session.y;
    let me_rotation = recipient_session.rotation;
    let me_movement_state = recipient_session.movement_state;
    let me_weapon_stats = recipient_session.weapon.stats();

    let mut viewer_sight = Sight::player();
    viewer_sight.direction = decode_rotation(me_rotation);
    viewer_sight.range = me_weapon_stats.visibility_range;
    viewer_sight.half_angle = me_weapon_stats.visibility_half_angle_deg.to_radians();
    let viewer_pos = (me_x, me_y);

    let players: Vec<PlayerState> = st
        .sessions
        .iter()
        .filter_map(|(&addr, session)| {
            if addr == recipient_addr || session.health == 0 {
                return None;
            }
            if !viewer_sight.can_see(viewer_pos, (session.x, session.y), walls) {
                return None;
            }
            Some(PlayerState {
                id: session.player_id,
                x: session.x,
                y: session.y,
                rotation: session.rotation,
                movement_state: session.movement_state,
                weapon: 0,
            })
        })
        .collect();
    ServerPacket {
        tick,
        timestamp: recipient_session
            .latest_input
            .as_ref()
            .map(|pkt| pkt.timestamp)
            .unwrap_or(0),
        me: SelfState {
            health: recipient_session.health.min(u8::MAX as u16) as u8,
            ammo: recipient_session.weapon.ammo_in_mag().min(u8::MAX as u32) as u8,
            // Non-zero while reloading (exact progress tracked client-side).
            reload_progress: if recipient_session.weapon.is_reloading() {
                128
            } else {
                0
            },
            movement_state: me_movement_state,
            x: me_x,
            y: me_y,
            rotation: me_rotation,
            aim_cone_half_angle: recipient_session.aim_cone_half_angle,
        },
        players,
        // Send every bullet to every client; the client decides whether to
        // render based on local visibility (see bullet_event_visible_to_local_player).
        bullets: bullets.to_vec(),
        sounds: vec![],
        match_state: MatchState {
            timer: 300,
            score_team1: 0,
            score_team2: 0,
        },
    }
}

fn apply_session_input(
    session: &mut Session,
    input: &ClientPacket,
    dt: f32,
    walls: &[Wall],
    level_bounds: Option<LevelBounds>,
) {
    let speed = player_speed_from_input(input);
    let mut dx = input.movement_x as f32;
    let mut dy = input.movement_y as f32;
    let len = (dx * dx + dy * dy).sqrt();
    if len > 1.0 {
        dx /= len;
        dy /= len;
    }

    session.x += dx * speed * dt;
    session.y += dy * speed * dt;
    wall::resolve_all(&mut session.x, &mut session.y, PLAYER_HALF, walls);
    clamp_actor_to_level_bounds(&mut session.x, &mut session.y, PLAYER_HALF, level_bounds);
    session.rotation = input.rotation;
    session.movement_state = movement_state_from_input(Some(input), false);
}

fn player_speed_from_input(input: &ClientPacket) -> f32 {
    match input.flags.move_speed() {
        MoveSpeed::SlowWalk => WALK_SPEED,
        MoveSpeed::Walk => NORMAL_SPEED,
        MoveSpeed::Run => RUN_SPEED,
    }
}

fn movement_state_from_input(
    latest_input: Option<&ClientPacket>,
    is_reloading: bool,
) -> MovementState {
    let mut state = MovementState(0);
    state.set_move_speed(
        latest_input
            .map(|pkt| pkt.flags.move_speed())
            .unwrap_or(MoveSpeed::Walk),
    );
    state.set_peeking(
        latest_input
            .map(|pkt| pkt.flags.is_peeking())
            .unwrap_or(false),
    );
    state.set_reloading(is_reloading);
    state
}

fn clamp_actor_to_level_bounds(x: &mut f32, y: &mut f32, half: f32, bounds: Option<LevelBounds>) {
    let Some(bounds) = bounds else {
        return;
    };
    let min_x = bounds.x + half;
    let max_x = bounds.x + bounds.w - half;
    let min_y = bounds.y + half;
    let max_y = bounds.y + bounds.h - half;

    if min_x <= max_x {
        *x = x.clamp(min_x, max_x);
    } else {
        *x = bounds.x + bounds.w * 0.5;
    }
    if min_y <= max_y {
        *y = y.clamp(min_y, max_y);
    } else {
        *y = bounds.y + bounds.h * 0.5;
    }
}

fn step_combat(st: &mut ServerState, walls: &mut Vec<Wall>, dt: f32) -> Vec<BulletEvent> {
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

        let mut hit_player_id = 0;
        if let Some(target_addr) = hit_target_addr {
            if let Some(target) = st.sessions.get_mut(&target_addr) {
                let damage = damage.min(u16::MAX as u32) as u16;
                target.health = target.health.saturating_sub(damage);
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
    shooter.aim_cone_half_angle = shooter.weapon.stats().aim_base_half_angle_deg.to_radians();

    if !latest_input.flags.is_shooting() {
        return None;
    }

    let weapon_stats = shooter.weapon.stats();
    if !shooter.weapon.try_fire_with_stats(weapon_stats) {
        return None;
    }

    let theta = decode_rotation(shooter.rotation);
    let dir = (theta.cos(), theta.sin());
    Some((
        shooter.player_id,
        (shooter.x, shooter.y),
        dir,
        weapon_stats.bullet_damage,
    ))
}

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

fn default_weapon_state() -> WeaponState {
    let default_weapon = all_weapon_ids()
        .first()
        .copied()
        .expect("weapon catalog is empty");
    WeaponState::new(default_weapon)
}

fn spawn_offset(player_id: u16) -> (f32, f32) {
    if player_id <= 1 {
        return (0.0, 0.0);
    }
    let idx = (player_id - 1) as f32;
    let angle = idx * 2.3999632;
    let radius = px_to_tiles(26.0);
    (angle.cos() * radius, angle.sin() * radius)
}

fn server_time_ms() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u32
}

/// Load the first level file from `assets/levels/`, sorted alphabetically —
/// mirrors the same logic used by the client so both start with identical
/// player spawn, walls, and map bounds.
fn load_first_level() -> Option<LevelData> {
    let dir = std::fs::read_dir("assets/levels").ok()?;
    let mut paths: Vec<_> = dir
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("json"))
        .collect();
    paths.sort();
    paths
        .first()
        .and_then(|p| p.to_str())
        .and_then(|s| LevelData::load(s).ok())
}
