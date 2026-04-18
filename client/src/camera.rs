use game::render::screen::ScreenTransform;
use game::world::rooms::LevelRooms;
use game::world::units::{px_to_tiles, tiles_to_px};

const GAP_NEAR_RADIUS: f32 = px_to_tiles(180.0);

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
    pub viewport_px: (f32, f32),
    pub player_pos: (f32, f32),
    pub bounds: CameraBounds,
    pub rooms: &'a LevelRooms,
    pub desired_state: CameraBehaviorState,
}

pub struct TacticalCamera {
    center: (f32, f32),
    state: CameraBehaviorState,
    in_room: bool,
    near_gap: bool,
}

impl TacticalCamera {
    pub fn new(initial_center: (f32, f32)) -> Self {
        Self {
            center: initial_center,
            state: CameraBehaviorState::Exploration,
            in_room: false,
            near_gap: false,
        }
    }

    pub fn reset(&mut self, center: (f32, f32)) {
        self.center = center;
        self.state = CameraBehaviorState::Exploration;
        self.in_room = false;
        self.near_gap = false;
    }

    pub fn center(&self) -> (f32, f32) {
        self.center
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
        self.screen_transform(viewport_px).world_to_screen(world)
    }

    pub fn world_points_to_screen<I>(&self, points: I, viewport_px: (f32, f32)) -> Vec<[f32; 2]>
    where
        I: IntoIterator<Item = [f32; 2]>,
    {
        self.screen_transform(viewport_px)
            .world_points_to_screen(points)
    }

    pub fn screen_to_world(&self, screen_px: (f32, f32), viewport_px: (f32, f32)) -> (f32, f32) {
        self.screen_transform(viewport_px)
            .screen_to_world(screen_px)
    }

    pub fn update(&mut self, step: CameraStepInput<'_>) {
        let player_pos = step.player_pos;

        // HUD metadata — room / gap detection.
        let room_id = step.rooms.find_room_at(player_pos.0, player_pos.1);
        self.in_room = room_id.is_some();
        self.near_gap = is_near_gap(step.rooms, room_id, player_pos);

        self.state = step.desired_state;
        if self.near_gap && self.state == CameraBehaviorState::Exploration {
            self.state = CameraBehaviorState::PeekTension;
        }

        // Camera snaps directly to the player — no lag, no offset.
        self.center = step.bounds.clamp_center(player_pos, step.viewport_px);
    }

    pub fn screen_transform(&self, viewport_px: (f32, f32)) -> ScreenTransform {
        ScreenTransform::new(self.center, viewport_px, tiles_to_px(1.0))
    }
}

fn is_near_gap(rooms: &LevelRooms, room_id: Option<usize>, player_pos: (f32, f32)) -> bool {
    let radius_sq = GAP_NEAR_RADIUS * GAP_NEAR_RADIUS;

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
            if dx * dx + dy * dy <= radius_sq {
                return true;
            }
        }
    }

    for &(gx, gy) in &rooms.gaps {
        let dx = gx - player_pos.0;
        let dy = gy - player_pos.1;
        if dx * dx + dy * dy <= radius_sq {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use game::world::rooms::{DetectedRoom, RoomGap};

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
            viewport_px: (1280.0, 720.0),
            player_pos: (4.0, 1.1),
            bounds: CameraBounds::from_min_max(0.0, 0.0, 20.0, 20.0),
            rooms: &rooms,
            desired_state: CameraBehaviorState::Exploration,
        });

        assert_eq!(cam.state(), CameraBehaviorState::PeekTension);
        assert!(cam.near_gap());
    }
}
