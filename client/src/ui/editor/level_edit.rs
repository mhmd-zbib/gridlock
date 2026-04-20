use std::cmp::Ordering;
use std::collections::HashSet;

use game::world::floor::LevelFloor;
use game::world::level::Pos;
use game::world::prop::LevelProp;
use game::world::units::px_to_tiles;
use game::world::wall::Wall;

use super::helpers::{
    build_corner_patch, choose_corner_cell, collect_edge_path, dist, edge_to_wall_rect,
    nearest_idx, point_to_segment_dist,
};
use super::tool::{BREAKABLE_HP, EDGE_STEP, EdgeAxis, EdgeCell, EdgeKey};
use super::{Editor, SnapMode, Tool};

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
                self.level.props.push(LevelProp { x, y, id: prop_id });
            }
            Tool::Floor => {
                let Some(asset) = self.selected_floor_definition() else {
                    return;
                };
                let floor_id = asset.id.clone();
                self.level.floors.push(LevelFloor { x, y, id: floor_id });
            }
            Tool::Wall | Tool::Breakable | Tool::BaseMap => {}
        }
    }

    pub(super) fn delete_at(&mut self, x: f32, y: f32) {
        const EDGE_REMOVE_RADIUS: f32 = px_to_tiles(18.0);
        if let Some((edge, _cell)) = self.nearest_edge_at(x, y, EDGE_REMOVE_RADIUS) {
            self.edges.remove(&edge);
            self.rebuild_walls_from_edges();
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

        const R: f32 = px_to_tiles(20.0);
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

    pub(super) fn add_edge_path(
        &mut self,
        start: (f32, f32),
        end: (f32, f32),
        mode: SnapMode,
        breakable: bool,
        thickness_steps: u32,
    ) -> Option<(usize, f32)> {
        let (keys, len) = collect_edge_path(start, end, mode)?;
        let mut added = 0usize;
        let cell = EdgeCell {
            breakable,
            hp: if breakable { BREAKABLE_HP } else { 1 },
            thickness_steps: thickness_steps.max(1),
        };
        for key in keys {
            if self.edges.insert(key, cell).is_none() {
                added += 1;
            }
        }
        self.rebuild_walls_from_edges();
        Some((added, len))
    }

    pub(super) fn rebuild_walls_from_edges(&mut self) {
        let mut out = Vec::new();
        let mut nodes = HashSet::new();
        for (key, cell) in &self.edges {
            let thickness = cell.thickness_steps as f32 * EDGE_STEP;
            let (x, y, w, h) = edge_to_wall_rect(*key, thickness);
            let horiz = key.axis == EdgeAxis::Horizontal;
            let mut wall = if cell.breakable {
                Wall::new_breakable(x, y, w, h, cell.hp)
            } else {
                Wall::new(x, y, w, h)
            };
            wall.horizontal = Some(horiz);
            out.push(wall);
            match key.axis {
                EdgeAxis::Horizontal => {
                    nodes.insert((key.x, key.y));
                    nodes.insert((key.x + 1, key.y));
                }
                EdgeAxis::Vertical => {
                    nodes.insert((key.x, key.y));
                    nodes.insert((key.x, key.y + 1));
                }
            }
        }

        for (vx, vy) in nodes {
            let west = self.edges.get(&EdgeKey {
                axis: EdgeAxis::Horizontal,
                x: vx - 1,
                y: vy,
            });
            let east = self.edges.get(&EdgeKey {
                axis: EdgeAxis::Horizontal,
                x: vx,
                y: vy,
            });
            let north = self.edges.get(&EdgeKey {
                axis: EdgeAxis::Vertical,
                x: vx,
                y: vy - 1,
            });
            let south = self.edges.get(&EdgeKey {
                axis: EdgeAxis::Vertical,
                x: vx,
                y: vy,
            });
            let has_h = west.is_some() || east.is_some();
            let has_v = north.is_some() || south.is_some();
            if has_h && has_v {
                let mut touching = Vec::new();
                if let Some(c) = west {
                    touching.push(*c);
                }
                if let Some(c) = east {
                    touching.push(*c);
                }
                if let Some(c) = north {
                    touching.push(*c);
                }
                if let Some(c) = south {
                    touching.push(*c);
                }
                out.push(build_corner_patch(vx, vy, choose_corner_cell(&touching)));
            }
        }

        self.level.walls = out;
    }

    pub(super) fn rebuild_edges_from_walls(&mut self) {
        self.edges.clear();
        for w in &self.level.walls {
            let (horizontal, is_new_format) = match w.horizontal {
                Some(h) => (h, true),
                None => {
                    if (w.w - w.h).abs() < 0.001 {
                        continue; // corner patch, skip — regenerated by rebuild_walls_from_edges
                    }
                    (w.w > w.h, false)
                }
            };
            let thickness_steps = if horizontal {
                ((w.h / EDGE_STEP).round() as u32).max(1)
            } else {
                ((w.w / EDGE_STEP).round() as u32).max(1)
            };
            let cell = EdgeCell {
                breakable: w.breakable,
                hp: w.hp.max(1),
                thickness_steps,
            };
            if horizontal {
                let y_line = if is_new_format { w.y } else { w.y + w.h * 0.5 };
                let y = (y_line / EDGE_STEP).round() as i32;
                let x0 = (w.x / EDGE_STEP).round() as i32;
                let x1 = ((w.x + w.w) / EDGE_STEP).round() as i32;
                let (from, to) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
                for x in from..to {
                    self.edges.insert(EdgeKey { axis: EdgeAxis::Horizontal, x, y }, cell);
                }
            } else {
                let x_line = if is_new_format { w.x } else { w.x + w.w * 0.5 };
                let x = (x_line / EDGE_STEP).round() as i32;
                let y0 = (w.y / EDGE_STEP).round() as i32;
                let y1 = ((w.y + w.h) / EDGE_STEP).round() as i32;
                let (from, to) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };
                for y in from..to {
                    self.edges.insert(EdgeKey { axis: EdgeAxis::Vertical, x, y }, cell);
                }
            }
        }
    }

    fn nearest_edge_at(&self, px: f32, py: f32, max_dist: f32) -> Option<(EdgeKey, EdgeCell)> {
        self.edges
            .iter()
            .map(|(key, cell)| {
                let (ax, ay, bx, by) = match key.axis {
                    EdgeAxis::Horizontal => {
                        let x0 = key.x as f32 * EDGE_STEP;
                        let y = key.y as f32 * EDGE_STEP;
                        (x0, y, x0 + EDGE_STEP, y)
                    }
                    EdgeAxis::Vertical => {
                        let x = key.x as f32 * EDGE_STEP;
                        let y0 = key.y as f32 * EDGE_STEP;
                        (x, y0, x, y0 + EDGE_STEP)
                    }
                };
                let d = point_to_segment_dist(px, py, ax, ay, bx, by);
                (*key, *cell, d)
            })
            .filter(|(_, _, d)| *d <= max_dist)
            .min_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(Ordering::Equal))
            .map(|(key, cell, _)| (key, cell))
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
