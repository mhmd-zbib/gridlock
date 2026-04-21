use std::cmp::Ordering;

use game::world::floor::LevelFloor;
use game::world::level::Pos;
use game::world::prop::LevelProp;
use game::world::units::px_to_tiles;
use game::world::wall::Wall;

use super::helpers::{dist, nearest_idx};
use super::tool::BREAKABLE_HP;
use super::{Editor, Tool};

impl Editor {
    pub(super) fn place(&mut self, x: f32, y: f32) {
        match self.tool {
            Tool::PlayerSpawn => {
                self.level.player_spawn = Some(Pos { x, y });
            }
            Tool::Team1Spawn => {
                self.level.team1_spawn = Some(Pos { x, y });
            }
            Tool::Team2Spawn => {
                self.level.team2_spawn = Some(Pos { x, y });
            }
            Tool::Enemy => {
                self.level.enemies.push(Pos { x, y });
            }
            Tool::TargetDummy => {
                self.level.target_enemies.push(Pos { x, y });
            }
            Tool::Prop => {
                let Some(asset) = self.selected_prop_definition() else {
                    return;
                };
                let prop_id = asset.id.clone();
                self.level.props.retain(|p| (p.x - x).abs() >= 0.001 || (p.y - y).abs() >= 0.001);
                self.level.props.push(LevelProp { x, y, id: prop_id });
            }
            Tool::Floor => {
                let Some(asset) = self.selected_floor_definition() else {
                    return;
                };
                let floor_id = asset.id.clone();
                self.level.floors.retain(|f| (f.x - x).abs() >= 0.001 || (f.y - y).abs() >= 0.001);
                self.level.floors.push(LevelFloor { x, y, id: floor_id });
            }
            Tool::Wall | Tool::Breakable | Tool::BaseMap | Tool::Eraser => {}
        }
    }

    pub(super) fn delete_at(&mut self, x: f32, y: f32) {
        const R: f32 = px_to_tiles(20.0);

        let found_wall = self
            .level
            .walls
            .iter()
            .enumerate()
            .map(|(i, w)| {
                let cx = w.x + w.w * 0.5;
                let cy = w.y + w.h * 0.5;
                (i, dist(cx, cy, x, y))
            })
            .filter(|(_, d)| *d < R)
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal))
            .map(|(i, _)| i);
        if let Some(idx) = found_wall {
            self.level.walls.remove(idx);
            return;
        }

        const FLOOR_REMOVE_RADIUS: f32 = px_to_tiles(20.0);
        if let Some(idx) = self.nearest_floor_idx(x, y, FLOOR_REMOVE_RADIUS) {
            self.level.floors.remove(idx);
            return;
        }

        const PROP_REMOVE_RADIUS: f32 = px_to_tiles(20.0);
        if let Some(idx) = self.nearest_prop_idx(x, y, PROP_REMOVE_RADIUS) {
            self.level.props.remove(idx);
            return;
        }

        if let Some(idx) = nearest_idx(&self.level.enemies, x, y, R) {
            self.level.enemies.remove(idx);
            return;
        }
        if let Some(idx) = nearest_idx(&self.level.target_enemies, x, y, R) {
            self.level.target_enemies.remove(idx);
            return;
        }
        if let Some(sp) = self.level.player_spawn {
            if dist(sp.x, sp.y, x, y) < R {
                self.level.player_spawn = None;
                return;
            }
        }
        if let Some(sp) = self.level.team1_spawn {
            if dist(sp.x, sp.y, x, y) < R {
                self.level.team1_spawn = None;
                return;
            }
        }
        if let Some(sp) = self.level.team2_spawn {
            if dist(sp.x, sp.y, x, y) < R {
                self.level.team2_spawn = None;
            }
        }
    }

    /// Place a single EDGE_STEP cell. All stored walls are always this atomic size.
    fn place_wall_cell(&mut self, x: f32, y: f32, breakable: bool) {
        use super::tool::EDGE_STEP;
        self.level.walls.retain(|w| (w.x - x).abs() >= 0.001 || (w.y - y).abs() >= 0.001);
        let wall = if breakable {
            Wall::new_breakable(x, y, EDGE_STEP, EDGE_STEP, BREAKABLE_HP)
        } else {
            Wall::new(x, y, EDGE_STEP, EDGE_STEP)
        };
        self.level.walls.push(wall);
    }

    /// Fill a brush-sized square of EDGE_STEP cells starting at (cx, cy).
    pub(super) fn place_wall_block(&mut self, cx: f32, cy: f32, breakable: bool) {
        use super::tool::EDGE_STEP;
        for iy in 0..self.wall_size_steps {
            for ix in 0..self.wall_size_steps {
                self.place_wall_cell(
                    cx + ix as f32 * EDGE_STEP,
                    cy + iy as f32 * EDGE_STEP,
                    breakable,
                );
            }
        }
    }

    /// Place a line of brush-sized blocks from `start` to `end` (both already snapped).
    /// The line is forced to be axis-aligned (H or V based on which delta is larger).
    pub(super) fn place_wall_line(&mut self, start: (f32, f32), end: (f32, f32), breakable: bool) {
        let size = self.wall_block_size();
        let (sx, sy) = start;
        let (ex, ey) = end;

        if (ex - sx).abs() >= (ey - sy).abs() {
            let (from_x, to_x) = if sx <= ex { (sx, ex) } else { (ex, sx) };
            let mut x = from_x;
            while x <= to_x + 0.001 {
                self.place_wall_block(x, sy, breakable);
                x += size;
            }
        } else {
            let (from_y, to_y) = if sy <= ey { (sy, ey) } else { (ey, sy) };
            let mut y = from_y;
            while y <= to_y + 0.001 {
                self.place_wall_block(sx, y, breakable);
                y += size;
            }
        }
    }

    pub(super) fn erase_in_rect(&mut self, cx: f32, cy: f32, half: f32) {
        let (x0, x1, y0, y1) = (cx - half, cx + half, cy - half, cy + half);

        self.level.walls.retain(|w| {
            let wcx = w.x + w.w * 0.5;
            let wcy = w.y + w.h * 0.5;
            wcx < x0 || wcx > x1 || wcy < y0 || wcy > y1
        });

        self.level
            .enemies
            .retain(|p| p.x < x0 || p.x > x1 || p.y < y0 || p.y > y1);
        self.level
            .target_enemies
            .retain(|p| p.x < x0 || p.x > x1 || p.y < y0 || p.y > y1);
        self.level
            .props
            .retain(|p| p.x < x0 || p.x > x1 || p.y < y0 || p.y > y1);
        self.level
            .floors
            .retain(|p| p.x < x0 || p.x > x1 || p.y < y0 || p.y > y1);
        if let Some(sp) = self.level.player_spawn {
            if sp.x >= x0 && sp.x <= x1 && sp.y >= y0 && sp.y <= y1 {
                self.level.player_spawn = None;
            }
        }
        if let Some(sp) = self.level.team1_spawn {
            if sp.x >= x0 && sp.x <= x1 && sp.y >= y0 && sp.y <= y1 {
                self.level.team1_spawn = None;
            }
        }
        if let Some(sp) = self.level.team2_spawn {
            if sp.x >= x0 && sp.x <= x1 && sp.y >= y0 && sp.y <= y1 {
                self.level.team2_spawn = None;
            }
        }
    }

    pub(super) fn delete_wall_at_point(&mut self, cx: f32, cy: f32, breakable: bool) {
        use super::tool::EDGE_STEP;
        for iy in 0..self.wall_size_steps {
            for ix in 0..self.wall_size_steps {
                let x = cx + ix as f32 * EDGE_STEP;
                let y = cy + iy as f32 * EDGE_STEP;
                self.level.walls.retain(|w| {
                    w.breakable != breakable
                        || (w.x - x).abs() >= 0.001
                        || (w.y - y).abs() >= 0.001
                });
            }
        }
    }

    pub(super) fn delete_nearest_prop_at(&mut self, x: f32, y: f32) {
        const R: f32 = px_to_tiles(20.0);
        if let Some(idx) = self.nearest_prop_idx(x, y, R) {
            self.level.props.remove(idx);
        }
    }

    pub(super) fn delete_nearest_floor_at(&mut self, x: f32, y: f32) {
        const R: f32 = px_to_tiles(20.0);
        if let Some(idx) = self.nearest_floor_idx(x, y, R) {
            self.level.floors.remove(idx);
        }
    }

    pub(super) fn delete_nearest_enemy_at(&mut self, x: f32, y: f32) {
        const R: f32 = px_to_tiles(20.0);
        if let Some(idx) = nearest_idx(&self.level.enemies, x, y, R) {
            self.level.enemies.remove(idx);
        }
    }

    pub(super) fn delete_nearest_target_dummy_at(&mut self, x: f32, y: f32) {
        const R: f32 = px_to_tiles(20.0);
        if let Some(idx) = nearest_idx(&self.level.target_enemies, x, y, R) {
            self.level.target_enemies.remove(idx);
        }
    }

    pub(super) fn delete_player_spawn_at(&mut self, x: f32, y: f32) {
        const R: f32 = px_to_tiles(20.0);
        if let Some(sp) = self.level.player_spawn {
            if dist(sp.x, sp.y, x, y) < R {
                self.level.player_spawn = None;
            }
        }
    }

    pub(super) fn delete_team_spawn_at(&mut self, x: f32, y: f32, team: u8) {
        const R: f32 = px_to_tiles(20.0);
        let spawn = if team == 1 {
            &mut self.level.team1_spawn
        } else {
            &mut self.level.team2_spawn
        };
        if let Some(sp) = *spawn {
            if dist(sp.x, sp.y, x, y) < R {
                *spawn = None;
            }
        }
    }

    fn nearest_prop_idx(&self, x: f32, y: f32, max_dist: f32) -> Option<usize> {
        self.level
            .props
            .iter()
            .enumerate()
            .map(|(idx, prop)| (idx, dist(prop.x, prop.y, x, y)))
            .filter(|(_, d)| *d < max_dist)
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal))
            .map(|(idx, _)| idx)
    }

    fn nearest_floor_idx(&self, x: f32, y: f32, max_dist: f32) -> Option<usize> {
        self.level
            .floors
            .iter()
            .enumerate()
            .map(|(idx, floor)| (idx, dist(floor.x, floor.y, x, y)))
            .filter(|(_, d)| *d < max_dist)
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal))
            .map(|(idx, _)| idx)
    }
}
