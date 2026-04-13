use super::rooms::LevelRooms;
use super::units::{px_to_tiles, tiles_to_px};

const W_AIM: f32 = 0.68;
const W_MOVE: f32 = 0.32;
const MAX_OFFSET_RADIUS: f32 = px_to_tiles(200.0);
const OFFSET_RESPONSE_GAMMA: f32 = 1.35;
const AIM_DEADZONE: f32 = px_to_tiles(56.0);
const AIM_FULL_DISTANCE: f32 = px_to_tiles(420.0);
const AIM_RESPONSE_GAMMA: f32 = 1.45;
const GAP_NEAR_RADIUS: f32 = px_to_tiles(180.0);
const GAP_BIAS_STRENGTH: f32 = px_to_tiles(90.0);
const IN_ROOM_AIM_SCALE: f32 = 0.62;
const IN_ROOM_MOVE_SCALE: f32 = 0.70;
const VISIBILITY_DRIFT_FRAC: f32 = 0.38;
const MIN_VISIBILITY_DRIFT: f32 = px_to_tiles(96.0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CameraBehaviorState {
    Exploration,
    Combat,
    PeekTension,
}

#[derive(Clone, Copy, Debug)]
pub struct CameraBounds {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

impl CameraBounds {
    pub fn from_min_max(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> Self {
        Self {
            min_x: min_x.min(max_x),
            min_y: min_y.min(max_y),
            max_x: max_x.max(min_x),
            max_y: max_y.max(min_y),
        }
    }

    pub fn clamp_center(&self, center: (f32, f32), viewport_px: (f32, f32)) -> (f32, f32) {
        let half_w = px_to_tiles(viewport_px.0 * 0.5);
        let half_h = px_to_tiles(viewport_px.1 * 0.5);

        let min_cx = self.min_x + half_w;
        let max_cx = self.max_x - half_w;
        let min_cy = self.min_y + half_h;
        let max_cy = self.max_y - half_h;

        let cx = if min_cx <= max_cx {
            center.0.clamp(min_cx, max_cx)
        } else {
            (self.min_x + self.max_x) * 0.5
        };
        let cy = if min_cy <= max_cy {
            center.1.clamp(min_cy, max_cy)
        } else {
            (self.min_y + self.max_y) * 0.5
        };
        (cx, cy)
    }
}

pub struct CameraStepInput<'a> {
    pub dt: f32,
    pub viewport_px: (f32, f32),
    pub player_pos: (f32, f32),
    pub mouse_world: (f32, f32),
    pub player_vision_range: f32,
    pub bounds: CameraBounds,
    pub rooms: &'a LevelRooms,
    pub desired_state: CameraBehaviorState,
}

pub struct TacticalCamera {
    center: (f32, f32),
    offset: (f32, f32),
    last_player_pos: Option<(f32, f32)>,
    state: CameraBehaviorState,
    in_room: bool,
    near_gap: bool,
}

impl TacticalCamera {
    pub fn new(initial_center: (f32, f32)) -> Self {
        Self {
            center: initial_center,
            offset: (0.0, 0.0),
            last_player_pos: None,
            state: CameraBehaviorState::Exploration,
            in_room: false,
            near_gap: false,
        }
    }

    pub fn reset(&mut self, center: (f32, f32)) {
        self.center = center;
        self.offset = (0.0, 0.0);
        self.last_player_pos = None;
        self.state = CameraBehaviorState::Exploration;
        self.in_room = false;
        self.near_gap = false;
    }

    pub fn center(&self) -> (f32, f32) {
        self.center
    }

    pub fn offset(&self) -> (f32, f32) {
        self.offset
    }

    pub fn state(&self) -> CameraBehaviorState {
        self.state
    }

    pub fn in_room(&self) -> bool {
        self.in_room
    }

    pub fn near_gap(&self) -> bool {
        self.near_gap
    }

    pub fn world_to_screen(&self, world: (f32, f32), viewport_px: (f32, f32)) -> (f32, f32) {
        (
            tiles_to_px(world.0 - self.center.0) + viewport_px.0 * 0.5,
            tiles_to_px(world.1 - self.center.1) + viewport_px.1 * 0.5,
        )
    }

    pub fn screen_to_world(&self, screen_px: (f32, f32), viewport_px: (f32, f32)) -> (f32, f32) {
        (
            px_to_tiles(screen_px.0 - viewport_px.0 * 0.5) + self.center.0,
            px_to_tiles(screen_px.1 - viewport_px.1 * 0.5) + self.center.1,
        )
    }

    pub fn update(&mut self, step: CameraStepInput<'_>) {
        let dt = step.dt.max(1.0e-5);
        let player_pos = step.player_pos;

        let move_dir = match self.last_player_pos {
            Some(prev) => normalize(((player_pos.0 - prev.0) / dt, (player_pos.1 - prev.1) / dt)),
            None => (0.0, 0.0),
        };
        self.last_player_pos = Some(player_pos);

        let aim_delta = (
            step.mouse_world.0 - player_pos.0,
            step.mouse_world.1 - player_pos.1,
        );
        let aim_dir = normalize(aim_delta);
        let aim_scale = aim_distance_scale(length(aim_delta));

        let room_id = step.rooms.find_room_at(player_pos.0, player_pos.1);
        self.in_room = room_id.is_some();

        let (gap_dir, gap_pull, near_gap) = nearest_gap_bias(step.rooms, room_id, player_pos);
        self.near_gap = near_gap;
        self.state = if step.desired_state == CameraBehaviorState::Combat {
            CameraBehaviorState::Combat
        } else if near_gap {
            CameraBehaviorState::PeekTension
        } else {
            step.desired_state
        };

        let (follow_alpha, offset_alpha, mut k_aim, mut k_move) = match self.state {
            CameraBehaviorState::Combat => (0.58, 0.70, px_to_tiles(170.0), px_to_tiles(84.0)),
            CameraBehaviorState::PeekTension => (0.46, 0.60, px_to_tiles(140.0), px_to_tiles(72.0)),
            CameraBehaviorState::Exploration => (0.36, 0.50, px_to_tiles(115.0), px_to_tiles(56.0)),
        };

        if self.in_room {
            k_aim *= IN_ROOM_AIM_SCALE;
            k_move *= IN_ROOM_MOVE_SCALE;
        }

        let o_aim = (aim_dir.0 * k_aim * aim_scale, aim_dir.1 * k_aim * aim_scale);
        let o_move = (move_dir.0 * k_move, move_dir.1 * k_move);

        let mut desired_offset = (
            o_aim.0 * W_AIM + o_move.0 * W_MOVE,
            o_aim.1 * W_AIM + o_move.1 * W_MOVE,
        );
        if gap_pull > 0.0 {
            desired_offset.0 += gap_dir.0 * GAP_BIAS_STRENGTH * gap_pull;
            desired_offset.1 += gap_dir.1 * GAP_BIAS_STRENGTH * gap_pull;
        }

        desired_offset = apply_response_curve(desired_offset);
        self.offset = lerp2(self.offset, desired_offset, offset_alpha);

        let c_base = lerp2(self.center, player_pos, follow_alpha);
        let mut desired_center = (c_base.0 + self.offset.0, c_base.1 + self.offset.1);
        desired_center = clamp_visibility_region(
            desired_center,
            player_pos,
            step.player_vision_range,
            MAX_OFFSET_RADIUS,
        );
        desired_center = step.bounds.clamp_center(desired_center, step.viewport_px);

        self.center = lerp2(self.center, desired_center, follow_alpha);
        self.center = step.bounds.clamp_center(self.center, step.viewport_px);
    }
}

fn nearest_gap_bias(
    rooms: &LevelRooms,
    room_id: Option<usize>,
    player_pos: (f32, f32),
) -> ((f32, f32), f32, bool) {
    let mut nearest = None::<(f32, f32, f32)>;

    if !rooms.gap_edges.is_empty() {
        for edge in &rooms.gap_edges {
            let matches = match room_id {
                Some(rid) => edge.from_room == Some(rid) || edge.to_room == Some(rid),
                None => edge.from_room.is_none() || edge.to_room.is_none(),
            };
            if !matches {
                continue;
            }
            let dx = edge.pos.0 - player_pos.0;
            let dy = edge.pos.1 - player_pos.1;
            let d2 = dx * dx + dy * dy;
            match nearest {
                Some((_, _, best_d2)) if d2 >= best_d2 => {}
                _ => nearest = Some((dx, dy, d2)),
            }
        }
    }

    if nearest.is_none() {
        for &(gx, gy) in &rooms.gaps {
            let dx = gx - player_pos.0;
            let dy = gy - player_pos.1;
            let d2 = dx * dx + dy * dy;
            match nearest {
                Some((_, _, best_d2)) if d2 >= best_d2 => {}
                _ => nearest = Some((dx, dy, d2)),
            }
        }
    }

    let Some((dx, dy, d2)) = nearest else {
        return ((0.0, 0.0), 0.0, false);
    };
    let dist = d2.sqrt();
    if dist > GAP_NEAR_RADIUS {
        return ((0.0, 0.0), 0.0, false);
    }
    let dir = normalize((dx, dy));
    let pull = 1.0 - (dist / GAP_NEAR_RADIUS);
    (dir, pull, true)
}

fn clamp_visibility_region(
    center: (f32, f32),
    player_pos: (f32, f32),
    vision_range: f32,
    max_drift: f32,
) -> (f32, f32) {
    let allowed = (vision_range * VISIBILITY_DRIFT_FRAC)
        .max(MIN_VISIBILITY_DRIFT)
        .min(max_drift);
    clamp_to_radius(center, player_pos, allowed)
}

fn apply_response_curve(offset: (f32, f32)) -> (f32, f32) {
    let mag = length(offset);
    if mag <= f32::EPSILON {
        return (0.0, 0.0);
    }
    let norm = (mag / MAX_OFFSET_RADIUS).clamp(0.0, 1.0);
    let curved = norm.powf(OFFSET_RESPONSE_GAMMA) * MAX_OFFSET_RADIUS;
    let dir = (offset.0 / mag, offset.1 / mag);
    (dir.0 * curved, dir.1 * curved)
}

fn aim_distance_scale(aim_dist: f32) -> f32 {
    if aim_dist <= AIM_DEADZONE {
        return 0.0;
    }
    let span = (AIM_FULL_DISTANCE - AIM_DEADZONE).max(1.0e-4);
    let normalized = ((aim_dist - AIM_DEADZONE) / span).clamp(0.0, 1.0);
    normalized.powf(AIM_RESPONSE_GAMMA)
}

fn lerp2(a: (f32, f32), b: (f32, f32), t: f32) -> (f32, f32) {
    let t = t.clamp(0.0, 1.0);
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

fn clamp_to_radius(point: (f32, f32), center: (f32, f32), radius: f32) -> (f32, f32) {
    let dx = point.0 - center.0;
    let dy = point.1 - center.1;
    let dist = (dx * dx + dy * dy).sqrt();
    if dist <= radius || dist <= f32::EPSILON {
        return point;
    }
    let scale = radius / dist;
    (center.0 + dx * scale, center.1 + dy * scale)
}

fn length(v: (f32, f32)) -> f32 {
    (v.0 * v.0 + v.1 * v.1).sqrt()
}

fn normalize(v: (f32, f32)) -> (f32, f32) {
    let len = length(v);
    if len <= f32::EPSILON {
        (0.0, 0.0)
    } else {
        (v.0 / len, v.1 / len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::world::rooms::{DetectedRoom, RoomGap};

    #[test]
    fn world_screen_roundtrip_is_stable() {
        let cam = TacticalCamera::new((6.0, 4.0));
        let viewport = (1280.0, 720.0);
        let world = (8.25, 2.5);
        let screen = cam.world_to_screen(world, viewport);
        let back = cam.screen_to_world(screen, viewport);
        assert!((back.0 - world.0).abs() < 1.0e-4);
        assert!((back.1 - world.1).abs() < 1.0e-4);
    }

    #[test]
    fn bounds_clamp_center_for_small_maps() {
        let bounds = CameraBounds::from_min_max(0.0, 0.0, px_to_tiles(320.0), px_to_tiles(180.0));
        let clamped = bounds.clamp_center((99.0, 99.0), (1280.0, 720.0));
        assert!((clamped.0 - px_to_tiles(160.0)).abs() < 1.0e-4);
        assert!((clamped.1 - px_to_tiles(90.0)).abs() < 1.0e-4);
    }

    #[test]
    fn near_gap_promotes_peek_state() {
        let mut rooms = LevelRooms::default();
        rooms.rooms.push(DetectedRoom {
            id: 0,
            x: 0.0,
            y: 0.0,
            w: 8.0,
            h: 8.0,
            color: [0.0; 4],
            centroid: (4.0, 4.0),
            area_cells: 64,
        });
        rooms.gap_edges.push(RoomGap {
            from_room: Some(0),
            to_room: None,
            pos: (4.0, 0.6),
            width_cells: 2,
        });

        let mut cam = TacticalCamera::new((4.0, 4.0));
        cam.update(CameraStepInput {
            dt: 1.0 / 60.0,
            viewport_px: (1280.0, 720.0),
            player_pos: (4.0, 1.1),
            mouse_world: (4.0, 0.1),
            player_vision_range: px_to_tiles(420.0),
            bounds: CameraBounds::from_min_max(0.0, 0.0, 20.0, 20.0),
            rooms: &rooms,
            desired_state: CameraBehaviorState::Exploration,
        });

        assert_eq!(cam.state(), CameraBehaviorState::PeekTension);
        assert!(cam.near_gap());
    }

    #[test]
    fn aim_distance_scale_uses_deadzone_and_caps() {
        assert_eq!(aim_distance_scale(AIM_DEADZONE * 0.5), 0.0);
        assert_eq!(aim_distance_scale(AIM_DEADZONE), 0.0);
        assert!((aim_distance_scale(AIM_FULL_DISTANCE) - 1.0).abs() < 1.0e-4);
        assert!((aim_distance_scale(AIM_FULL_DISTANCE * 2.0) - 1.0).abs() < 1.0e-4);
    }
}
