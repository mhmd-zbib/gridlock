use engine::render::quad::QuadInstance;
use engine::input::InputState;
use game::world::level::{LevelBounds, LevelData, Pos};
use game::world::prop::{self, LevelProp, PropAssetDef};
use game::world::units::{px_to_tiles, tiles_to_px};
use game::world::wall::Wall;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

// ---------------------------------------------------------------------------
// Tool
// ---------------------------------------------------------------------------

#[derive(Default, Clone, Copy, PartialEq)]
pub enum Tool {
    #[default]
    PlayerSpawn, // key 1 — green
    Enemy,       // key 2 — red
    Wall,        // key 3 — brown, 2-point thin wall
    TargetDummy, // key 4 — yellow
    Breakable,   // key 5 — cyan, 2-point thin breakable wall
    Prop,        // key 6 — id-backed prop
    BaseMap,     // key 7 — 2-point map bounds rectangle
}

// ---------------------------------------------------------------------------
// Editor
// ---------------------------------------------------------------------------

const WALL_THICKNESS: f32 = 0.1;
const BREAKABLE_THICKNESS: f32 = WALL_THICKNESS;
const BREAKABLE_HP: u32 = 1;
const TILE_GRID: f32 = 1.0;
const SUBGRID_DIVISIONS: usize = 8;
const EDGE_STEP: f32 = TILE_GRID / SUBGRID_DIVISIONS as f32;
const GRID_LINE_THICKNESS: f32 = px_to_tiles(1.0);
const GRID_LINE_ALPHA: f32 = 0.16;
const SUBGRID_LINE_ALPHA: f32 = 0.08;
const EDITOR_MIN_ZOOM: f32 = 0.35;
const EDITOR_MAX_ZOOM: f32 = 3.5;
const EDITOR_KEY_ZOOM_STEP: f32 = 1.04;
const EDITOR_WHEEL_ZOOM_STEP: f32 = 0.12;
const EDITOR_PAN_STEP_PX: f32 = 20.0;

#[derive(Clone, Copy, PartialEq)]
pub enum SnapMode {
    Edge,
    Center,
    Subgrid,
}

impl SnapMode {
    fn toggle(self) -> Self {
        match self {
            SnapMode::Edge => SnapMode::Center,
            SnapMode::Center => SnapMode::Subgrid,
            SnapMode::Subgrid => SnapMode::Edge,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SnapMode::Edge => "Tile Edges",
            SnapMode::Center => "Tile Centers",
            SnapMode::Subgrid => "Tile Subgrid (1/8)",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum EdgeAxis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct EdgeKey {
    axis: EdgeAxis,
    x: i32,
    y: i32,
}

#[derive(Clone, Copy)]
struct EdgeCell {
    breakable: bool,
    hp: u32,
}

pub struct Editor {
    pub tool: Tool,
    pub level: LevelData,
    pub snap_mode: SnapMode,
    pub show_subgrid: bool,
    pub zoom: f32,
    prop_assets: Vec<PropAssetDef>,
    selected_prop_asset: usize,
    edges: HashMap<EdgeKey, EdgeCell>,
    view_origin: (f32, f32),

    /// First point for two-click wall placement.
    wall_start: Option<(f32, f32)>,
    /// First point for two-click breakable placement.
    breakable_start: Option<(f32, f32)>,
    /// First point for two-click base map bounds.
    base_map_start: Option<(f32, f32)>,

    // Edge detection
    prev_left: bool,
    prev_right: bool,
    prev_f5: bool,
    prev_key_l: bool,
    prev_key_1: bool,
    prev_key_2: bool,
    prev_key_3: bool,
    prev_key_4: bool,
    prev_key_5: bool,
    prev_key_6: bool,
    prev_key_7: bool,
    prev_key_q: bool,
    prev_key_e: bool,
    prev_key_g: bool,
    prev_key_h: bool,
}

impl Editor {
    pub fn new() -> Self {
        let prop_assets = prop::load_assets();
        Self {
            tool: Tool::default(),
            level: LevelData::default(),
            snap_mode: SnapMode::Edge,
            show_subgrid: true,
            zoom: 1.0,
            prop_assets,
            selected_prop_asset: 0,
            edges: HashMap::new(),
            view_origin: (0.0, 0.0),
            wall_start: None,
            breakable_start: None,
            base_map_start: None,
            prev_left: false,
            prev_right: false,
            prev_f5: false,
            prev_key_l: false,
            prev_key_1: false,
            prev_key_2: false,
            prev_key_3: false,
            prev_key_4: false,
            prev_key_5: false,
            prev_key_6: false,
            prev_key_7: false,
            prev_key_q: false,
            prev_key_e: false,
            prev_key_g: false,
            prev_key_h: false,
        }
    }

    pub fn refresh_prop_assets(&mut self) {
        self.prop_assets = prop::load_assets();
        if self.selected_prop_asset >= self.prop_assets.len() {
            self.selected_prop_asset = 0;
        }
    }

    pub fn prop_assets(&self) -> &[PropAssetDef] {
        &self.prop_assets
    }

    pub fn selected_prop_asset(&self) -> Option<&PropAssetDef> {
        self.selected_prop_definition()
    }

    pub fn selected_prop_asset_index(&self) -> usize {
        self.selected_prop_asset
    }

    fn screen_to_world(&self, sx: f32, sy: f32) -> (f32, f32) {
        (
            self.view_origin.0 + px_to_tiles(sx) / self.zoom,
            self.view_origin.1 + px_to_tiles(sy) / self.zoom,
        )
    }

    fn world_to_screen(&self, wx: f32, wy: f32) -> (f32, f32) {
        (
            tiles_to_px(wx - self.view_origin.0) * self.zoom,
            tiles_to_px(wy - self.view_origin.1) * self.zoom,
        )
    }

    fn world_len_to_screen(&self, world_len: f32) -> f32 {
        tiles_to_px(world_len).abs() * self.zoom
    }

    fn world_quad(
        &self,
        center: (f32, f32),
        half_size: (f32, f32),
        color: [f32; 4],
    ) -> QuadInstance {
        let (cx, cy) = self.world_to_screen(center.0, center.1);
        QuadInstance {
            center: [cx, cy],
            half_size: [
                self.world_len_to_screen(half_size.0),
                self.world_len_to_screen(half_size.1),
            ],
            color,
        }
    }

    pub fn update(&mut self, input: &InputState) {
        let mouse_px = (input.mouse_x as f32, input.mouse_y as f32);

        let mut zoom_factor = 1.0_f32;
        if input.key_equals {
            zoom_factor *= EDITOR_KEY_ZOOM_STEP;
        }
        if input.key_minus {
            zoom_factor /= EDITOR_KEY_ZOOM_STEP;
        }
        if input.mouse_wheel_y != 0.0 {
            zoom_factor *= (1.0 + input.mouse_wheel_y * EDITOR_WHEEL_ZOOM_STEP).max(0.2);
        }
        if (zoom_factor - 1.0).abs() > f32::EPSILON {
            let world_before = self.screen_to_world(mouse_px.0, mouse_px.1);
            self.zoom = (self.zoom * zoom_factor).clamp(EDITOR_MIN_ZOOM, EDITOR_MAX_ZOOM);
            self.view_origin = (
                world_before.0 - px_to_tiles(mouse_px.0) / self.zoom,
                world_before.1 - px_to_tiles(mouse_px.1) / self.zoom,
            );
        }

        let pan_step_world = px_to_tiles(EDITOR_PAN_STEP_PX) / self.zoom.max(0.001);
        if input.left {
            self.view_origin.0 -= pan_step_world;
        }
        if input.right {
            self.view_origin.0 += pan_step_world;
        }
        if input.up {
            self.view_origin.1 -= pan_step_world;
        }
        if input.down {
            self.view_origin.1 += pan_step_world;
        }

        let (raw_mx, raw_my) = self.screen_to_world(mouse_px.0, mouse_px.1);
        let snap_mode = effective_snap_mode(self.tool, self.snap_mode);
        let (mx, my) = snap_point(raw_mx, raw_my, snap_mode);

        let just_pressed = input.mouse_left && !self.prev_left;
        let just_right_pressed = input.mouse_right && !self.prev_right;

        // --- tool selection ---
        if input.key_1 && !self.prev_key_1 {
            self.tool = Tool::PlayerSpawn;
            self.wall_start = None;
            self.breakable_start = None;
            self.base_map_start = None;
        }
        if input.key_2 && !self.prev_key_2 {
            self.tool = Tool::Enemy;
            self.wall_start = None;
            self.breakable_start = None;
            self.base_map_start = None;
        }
        if input.key_3 && !self.prev_key_3 {
            self.tool = Tool::Wall;
            self.wall_start = None;
            self.breakable_start = None;
            self.base_map_start = None;
        }
        if input.key_4 && !self.prev_key_4 {
            self.tool = Tool::TargetDummy;
            self.wall_start = None;
            self.breakable_start = None;
            self.base_map_start = None;
        }
        if input.key_5 && !self.prev_key_5 {
            self.tool = Tool::Breakable;
            self.wall_start = None;
            self.breakable_start = None;
            self.base_map_start = None;
        }
        if input.key_6 && !self.prev_key_6 {
            self.tool = Tool::Prop;
            self.wall_start = None;
            self.breakable_start = None;
            self.base_map_start = None;
            self.announce_selected_prop_asset();
        }
        if input.key_7 && !self.prev_key_7 {
            self.tool = Tool::BaseMap;
            self.wall_start = None;
            self.breakable_start = None;
            self.base_map_start = None;
        }

        // --- grid snap toggle ---
        if input.key_g && !self.prev_key_g {
            self.snap_mode = self.snap_mode.toggle();
        }
        if input.key_h && !self.prev_key_h {
            self.show_subgrid = !self.show_subgrid;
        }
        if self.tool == Tool::Prop {
            if input.key_q && !self.prev_key_q {
                self.cycle_prop_asset(-1);
            }
            if input.key_e && !self.prev_key_e {
                self.cycle_prop_asset(1);
            }
        }

        // --- place ---
        match self.tool {
            Tool::Wall => {
                if just_pressed {
                    if let Some(start) = self.wall_start.take() {
                        self.add_edge_path(start, (mx, my), snap_mode, false);
                    } else {
                        self.wall_start = Some((mx, my));
                    }
                }
            }
            Tool::Breakable => {
                if just_pressed {
                    if let Some(start) = self.breakable_start.take() {
                        self.add_edge_path(start, (mx, my), snap_mode, true);
                    } else {
                        self.breakable_start = Some((mx, my));
                    }
                }
            }
            Tool::BaseMap => {
                if just_pressed {
                    if let Some(start) = self.base_map_start.take() {
                        if let Some(bounds) = bounds_from_points(start, (mx, my)) {
                            self.level.map_bounds = Some(bounds);
                        }
                    } else {
                        self.base_map_start = Some((mx, my));
                    }
                }
            }
            _ => {
                if just_pressed {
                    self.place(mx, my);
                }
            }
        }

        // --- delete on right-click ---
        if just_right_pressed {
            if self.tool == Tool::Wall {
                self.wall_start.take();
            } else if self.tool == Tool::Breakable {
                self.breakable_start.take();
            } else if self.tool == Tool::BaseMap {
                if self.base_map_start.take().is_none() {
                    self.level.map_bounds.take();
                }
            } else {
                self.delete_at(raw_mx, raw_my);
            }
        }

        // --- save / load ---
        if input.f5 && !self.prev_f5 {
            self.save("levels/level_2.json");
        }
        if input.key_l && !self.prev_key_l {
            self.load("levels/level_2.json");
        }

        // Update edge state.
        self.prev_left = input.mouse_left;
        self.prev_right = input.mouse_right;
        self.prev_f5 = input.f5;
        self.prev_key_l = input.key_l;
        self.prev_key_1 = input.key_1;
        self.prev_key_2 = input.key_2;
        self.prev_key_3 = input.key_3;
        self.prev_key_4 = input.key_4;
        self.prev_key_5 = input.key_5;
        self.prev_key_6 = input.key_6;
        self.prev_key_7 = input.key_7;
        self.prev_key_q = input.key_q;
        self.prev_key_e = input.key_e;
        self.prev_key_g = input.key_g;
        self.prev_key_h = input.key_h;
    }

    fn place(&mut self, x: f32, y: f32) {
        match self.tool {
            Tool::PlayerSpawn => {
                self.level.player_spawn = Some(Pos { x, y });
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
            Tool::Wall | Tool::Breakable | Tool::BaseMap => {}
        }
    }

    fn delete_at(&mut self, x: f32, y: f32) {
        const EDGE_REMOVE_RADIUS: f32 = px_to_tiles(18.0);
        if let Some((edge, _cell)) = self.nearest_edge_at(x, y, EDGE_REMOVE_RADIUS) {
            self.edges.remove(&edge);
            self.rebuild_walls_from_edges();
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
            }
        }
    }

    pub fn save(&self, path: &str) {
        let _ = self.level.save(path);
    }

    pub fn load(&mut self, path: &str) {
        if let Ok(data) = LevelData::load(path) {
            self.level = data;
            self.rebuild_edges_from_walls();
            self.rebuild_walls_from_edges();
        }
    }

    /// Build the instance list for the editor overlay.
    pub fn instances(
        &self,
        screen_w: f32,
        screen_h: f32,
        mouse_x: f32,
        mouse_y: f32,
    ) -> Vec<QuadInstance> {
        let (mouse_x, mouse_y) = self.screen_to_world(mouse_x, mouse_y);
        let snap_mode = effective_snap_mode(self.tool, self.snap_mode);
        let (mx, my) = snap_point(mouse_x, mouse_y, snap_mode);

        let mut out = Vec::new();
        append_grid_lines(
            &mut out,
            screen_w,
            screen_h,
            self.show_subgrid,
            self.view_origin,
            self.zoom,
        );

        if let Some(bounds) = self.level.map_bounds {
            push_bounds_fill(
                &mut out,
                bounds,
                [0.10, 0.16, 0.26, 0.16],
                [0.16, 0.42, 0.95, 0.72],
                self.view_origin,
                self.zoom,
            );
        }

        // Walls — brownish gray
        for w in &self.level.walls {
            out.push(self.world_quad(
                (w.x + w.w * 0.5, w.y + w.h * 0.5),
                (w.w * 0.5, w.h * 0.5),
                if w.breakable {
                    [0.2, 0.8, 0.95, 1.0]
                } else {
                    [0.45, 0.4, 0.35, 1.0]
                },
            ));
        }

        // Props
        for prop in &self.level.props {
            let (half_w, half_h, is_collider) = match self.find_prop_asset(&prop.id) {
                Some(asset) => (asset.width * 0.5, asset.height * 0.5, asset.is_collider),
                None => (px_to_tiles(6.0), px_to_tiles(6.0), false),
            };
            out.push(self.world_quad(
                (prop.x, prop.y),
                (half_w, half_h),
                prop::asset_color(&prop.id, is_collider, 1.0),
            ));
        }

        // Player spawn — green
        if let Some(sp) = self.level.player_spawn {
            out.push(self.world_quad(
                (sp.x, sp.y),
                (px_to_tiles(10.0), px_to_tiles(10.0)),
                [0.2, 1.0, 0.2, 1.0],
            ));
        }

        // Enemies — red
        for e in &self.level.enemies {
            out.push(self.world_quad(
                (e.x, e.y),
                (px_to_tiles(8.0), px_to_tiles(8.0)),
                [1.0, 0.2, 0.2, 1.0],
            ));
        }

        // Target dummies — yellow
        for e in &self.level.target_enemies {
            out.push(self.world_quad(
                (e.x, e.y),
                (px_to_tiles(8.0), px_to_tiles(8.0)),
                [1.0, 0.85, 0.2, 1.0],
            ));
        }

        // Ghost preview
        match self.tool {
            Tool::Wall => {
                if let Some(start) = self.wall_start {
                    for w in preview_edge_walls(start, (mx, my), snap_mode, false) {
                        out.push(self.world_quad(
                            (w.x + w.w * 0.5, w.y + w.h * 0.5),
                            (w.w * 0.5, w.h * 0.5),
                            [0.45, 0.4, 0.35, 0.45],
                        ));
                    }
                } else {
                    out.push(self.world_quad(
                        (mx, my),
                        (px_to_tiles(3.0), px_to_tiles(3.0)),
                        [0.45, 0.4, 0.35, 0.5],
                    ));
                }
            }
            Tool::Breakable => {
                if let Some(start) = self.breakable_start {
                    for w in preview_edge_walls(start, (mx, my), snap_mode, true) {
                        out.push(self.world_quad(
                            (w.x + w.w * 0.5, w.y + w.h * 0.5),
                            (w.w * 0.5, w.h * 0.5),
                            [0.2, 0.8, 0.95, 0.45],
                        ));
                    }
                } else {
                    out.push(self.world_quad(
                        (mx, my),
                        (px_to_tiles(3.0), px_to_tiles(3.0)),
                        [0.2, 0.8, 0.95, 0.5],
                    ));
                }
            }
            Tool::PlayerSpawn => {
                out.push(self.world_quad(
                    (mx, my),
                    (px_to_tiles(10.0), px_to_tiles(10.0)),
                    [0.2, 1.0, 0.2, 0.35],
                ));
            }
            Tool::Enemy => {
                out.push(self.world_quad(
                    (mx, my),
                    (px_to_tiles(8.0), px_to_tiles(8.0)),
                    [1.0, 0.2, 0.2, 0.35],
                ));
            }
            Tool::TargetDummy => {
                out.push(self.world_quad(
                    (mx, my),
                    (px_to_tiles(8.0), px_to_tiles(8.0)),
                    [1.0, 0.85, 0.2, 0.35],
                ));
            }
            Tool::Prop => {
                if let Some(asset) = self.selected_prop_definition() {
                    out.push(self.world_quad(
                        (mx, my),
                        (asset.width * 0.5, asset.height * 0.5),
                        prop::asset_color(&asset.id, asset.is_collider, 0.45),
                    ));
                } else {
                    out.push(self.world_quad(
                        (mx, my),
                        (px_to_tiles(5.0), px_to_tiles(5.0)),
                        [0.95, 0.2, 0.2, 0.5],
                    ));
                }
            }
            Tool::BaseMap => {
                if let Some(start) = self.base_map_start {
                    if let Some(bounds) = bounds_from_points(start, (mx, my)) {
                        push_bounds_fill(
                            &mut out,
                            bounds,
                            [0.12, 0.22, 0.36, 0.28],
                            [0.35, 0.65, 1.0, 0.86],
                            self.view_origin,
                            self.zoom,
                        );
                    }
                } else {
                    out.push(self.world_quad(
                        (mx, my),
                        (px_to_tiles(4.0), px_to_tiles(4.0)),
                        [0.35, 0.65, 1.0, 0.55],
                    ));
                }
            }
        }

        out
    }

    pub fn active_snap_label(&self) -> &'static str {
        effective_snap_mode(self.tool, self.snap_mode).label()
    }

    fn selected_prop_definition(&self) -> Option<&PropAssetDef> {
        self.prop_assets.get(self.selected_prop_asset)
    }

    fn find_prop_asset(&self, id: &str) -> Option<&PropAssetDef> {
        self.prop_assets.iter().find(|asset| asset.id == id)
    }

    fn cycle_prop_asset(&mut self, dir: i32) {
        if self.prop_assets.is_empty() {
            return;
        }
        let len = self.prop_assets.len() as i32;
        self.selected_prop_asset =
            ((self.selected_prop_asset as i32 + dir).rem_euclid(len)) as usize;
        self.announce_selected_prop_asset();
    }

    fn announce_selected_prop_asset(&self) {
        // intentionally silent — selection is shown in the editor overlay
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

    fn add_edge_path(
        &mut self,
        start: (f32, f32),
        end: (f32, f32),
        mode: SnapMode,
        breakable: bool,
    ) -> Option<(usize, f32)> {
        let (keys, len) = collect_edge_path(start, end, mode)?;

        let mut added = 0usize;
        let cell = EdgeCell {
            breakable,
            hp: if breakable { BREAKABLE_HP } else { 1 },
        };
        for key in keys {
            if self.edges.insert(key, cell).is_none() {
                added += 1;
            }
        }

        self.rebuild_walls_from_edges();
        Some((added, len))
    }

    fn rebuild_walls_from_edges(&mut self) {
        let mut out = Vec::new();
        let mut nodes = HashSet::new();
        for (key, cell) in &self.edges {
            let (x, y, w, h) = edge_to_wall_rect(*key, cell.breakable);
            if cell.breakable {
                out.push(Wall::new_breakable(x, y, w, h, cell.hp));
            } else {
                out.push(Wall::new(x, y, w, h));
            }
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

        // Bevel-like corner fill: add local join patch at shared vertices.
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

    fn rebuild_edges_from_walls(&mut self) {
        self.edges.clear();
        for w in &self.level.walls {
            let horizontal = w.w >= w.h;
            let cell = EdgeCell {
                breakable: w.breakable,
                hp: w.hp.max(1),
            };
            if horizontal {
                let y_line = w.y + w.h * 0.5;
                let y = (y_line / EDGE_STEP).round() as i32;
                let x0 = (w.x / EDGE_STEP).round() as i32;
                let x1 = ((w.x + w.w) / EDGE_STEP).round() as i32;
                let (from, to) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
                for x in from..to {
                    self.edges.insert(
                        EdgeKey {
                            axis: EdgeAxis::Horizontal,
                            x,
                            y,
                        },
                        cell,
                    );
                }
            } else {
                let x_line = w.x + w.w * 0.5;
                let x = (x_line / EDGE_STEP).round() as i32;
                let y0 = (w.y / EDGE_STEP).round() as i32;
                let y1 = ((w.y + w.h) / EDGE_STEP).round() as i32;
                let (from, to) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };
                for y in from..to {
                    self.edges.insert(
                        EdgeKey {
                            axis: EdgeAxis::Vertical,
                            x,
                            y,
                        },
                        cell,
                    );
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
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn snap(v: f32, grid: f32) -> f32 {
    (v / grid).round() * grid
}

fn snap_point(x: f32, y: f32, mode: SnapMode) -> (f32, f32) {
    match mode {
        SnapMode::Edge => (snap(x, TILE_GRID), snap(y, TILE_GRID)),
        SnapMode::Center => (
            snap(x - TILE_GRID * 0.5, TILE_GRID) + TILE_GRID * 0.5,
            snap(y - TILE_GRID * 0.5, TILE_GRID) + TILE_GRID * 0.5,
        ),
        SnapMode::Subgrid => {
            let sub = TILE_GRID / SUBGRID_DIVISIONS as f32;
            (snap(x, sub), snap(y, sub))
        }
    }
}

fn effective_snap_mode(tool: Tool, mode: SnapMode) -> SnapMode {
    if matches!(tool, Tool::Wall | Tool::Breakable) && mode == SnapMode::Center {
        SnapMode::Edge
    } else {
        mode
    }
}

fn point_to_edge_vertex(p: (f32, f32), mode: SnapMode) -> GridVertex {
    let (sx, sy) = snap_point(p.0, p.1, mode);
    GridVertex {
        x: (sx / EDGE_STEP).round() as i32,
        y: (sy / EDGE_STEP).round() as i32,
    }
}

fn edge_vertex_to_point(v: GridVertex) -> (f32, f32) {
    (v.x as f32 * EDGE_STEP, v.y as f32 * EDGE_STEP)
}

fn collect_edge_path(
    start: (f32, f32),
    end: (f32, f32),
    mode: SnapMode,
) -> Option<(Vec<EdgeKey>, f32)> {
    let mut a = point_to_edge_vertex(start, mode);
    let mut b = point_to_edge_vertex(end, mode);

    let dx = b.x - a.x;
    let dy = b.y - a.y;
    if dx.abs() >= dy.abs() {
        b.y = a.y;
    } else {
        b.x = a.x;
    }

    if a.x == b.x && a.y == b.y {
        return None;
    }

    if a.x > b.x || (a.x == b.x && a.y > b.y) {
        std::mem::swap(&mut a, &mut b);
    }

    let mut keys = Vec::new();
    if a.y == b.y {
        for x in a.x..b.x {
            keys.push(EdgeKey {
                axis: EdgeAxis::Horizontal,
                x,
                y: a.y,
            });
        }
    } else {
        for y in a.y..b.y {
            keys.push(EdgeKey {
                axis: EdgeAxis::Vertical,
                x: a.x,
                y,
            });
        }
    }

    let pa = edge_vertex_to_point(a);
    let pb = edge_vertex_to_point(b);
    Some((keys, dist(pa.0, pa.1, pb.0, pb.1)))
}

fn edge_to_wall_rect(edge: EdgeKey, breakable: bool) -> (f32, f32, f32, f32) {
    let thickness = if breakable {
        BREAKABLE_THICKNESS
    } else {
        WALL_THICKNESS
    };
    match edge.axis {
        EdgeAxis::Horizontal => (
            edge.x as f32 * EDGE_STEP,
            edge.y as f32 * EDGE_STEP - thickness * 0.5,
            EDGE_STEP,
            thickness,
        ),
        EdgeAxis::Vertical => (
            edge.x as f32 * EDGE_STEP - thickness * 0.5,
            edge.y as f32 * EDGE_STEP,
            thickness,
            EDGE_STEP,
        ),
    }
}

fn choose_corner_cell(cells: &[EdgeCell]) -> EdgeCell {
    let all_breakable = cells.iter().all(|c| c.breakable);
    if all_breakable {
        let hp = cells.iter().map(|c| c.hp).min().unwrap_or(1).max(1);
        EdgeCell {
            breakable: true,
            hp,
        }
    } else {
        EdgeCell {
            breakable: false,
            hp: 1,
        }
    }
}

fn build_corner_patch(vx: i32, vy: i32, cell: EdgeCell) -> Wall {
    let t = if cell.breakable {
        BREAKABLE_THICKNESS
    } else {
        WALL_THICKNESS
    };
    let x = vx as f32 * EDGE_STEP - t * 0.5;
    let y = vy as f32 * EDGE_STEP - t * 0.5;
    if cell.breakable {
        Wall::new_breakable(x, y, t, t, cell.hp)
    } else {
        Wall::new(x, y, t, t)
    }
}

fn preview_edge_walls(
    start: (f32, f32),
    end: (f32, f32),
    mode: SnapMode,
    breakable: bool,
) -> Vec<Wall> {
    let Some((keys, _)) = collect_edge_path(start, end, mode) else {
        return Vec::new();
    };
    keys.into_iter()
        .map(|k| {
            let (x, y, w, h) = edge_to_wall_rect(k, breakable);
            if breakable {
                Wall::new_breakable(x, y, w, h, BREAKABLE_HP)
            } else {
                Wall::new(x, y, w, h)
            }
        })
        .collect()
}

#[derive(Clone, Copy)]
struct GridVertex {
    x: i32,
    y: i32,
}

fn dist(ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let dx = ax - bx;
    let dy = ay - by;
    (dx * dx + dy * dy).sqrt()
}

fn point_to_segment_dist(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let vx = bx - ax;
    let vy = by - ay;
    let wx = px - ax;
    let wy = py - ay;
    let c1 = vx * wx + vy * wy;
    if c1 <= 0.0 {
        return dist(px, py, ax, ay);
    }
    let c2 = vx * vx + vy * vy;
    if c2 <= c1 {
        return dist(px, py, bx, by);
    }
    let t = c1 / c2;
    let qx = ax + t * vx;
    let qy = ay + t * vy;
    dist(px, py, qx, qy)
}

fn nearest_idx(points: &[Pos], x: f32, y: f32, max_dist: f32) -> Option<usize> {
    points
        .iter()
        .enumerate()
        .map(|(i, p)| (i, dist(p.x, p.y, x, y)))
        .filter(|(_, d)| *d < max_dist)
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .map(|(i, _)| i)
}

fn bounds_from_points(a: (f32, f32), b: (f32, f32)) -> Option<LevelBounds> {
    let min_x = a.0.min(b.0);
    let min_y = a.1.min(b.1);
    let max_x = a.0.max(b.0);
    let max_y = a.1.max(b.1);
    let w = max_x - min_x;
    let h = max_y - min_y;
    if w < EDGE_STEP || h < EDGE_STEP {
        return None;
    }
    Some(LevelBounds {
        x: min_x,
        y: min_y,
        w,
        h,
    })
}

fn push_bounds_fill(
    out: &mut Vec<QuadInstance>,
    bounds: LevelBounds,
    fill_color: [f32; 4],
    border_color: [f32; 4],
    view_origin: (f32, f32),
    zoom: f32,
) {
    let scale = tiles_to_px(1.0) * zoom;
    let x = (bounds.x - view_origin.0) * scale;
    let y = (bounds.y - view_origin.1) * scale;
    let w = bounds.w * scale;
    let h = bounds.h * scale;
    let center_x = x + w * 0.5;
    let center_y = y + h * 0.5;
    let border = 1.5;

    out.push(QuadInstance {
        center: [center_x, center_y],
        half_size: [w * 0.5, h * 0.5],
        color: fill_color,
    });

    out.push(QuadInstance {
        center: [center_x, y + border * 0.5],
        half_size: [w * 0.5, border * 0.5],
        color: border_color,
    });
    out.push(QuadInstance {
        center: [center_x, y + h - border * 0.5],
        half_size: [w * 0.5, border * 0.5],
        color: border_color,
    });
    out.push(QuadInstance {
        center: [x + border * 0.5, center_y],
        half_size: [border * 0.5, h * 0.5],
        color: border_color,
    });
    out.push(QuadInstance {
        center: [x + w - border * 0.5, center_y],
        half_size: [border * 0.5, h * 0.5],
        color: border_color,
    });
}

fn append_grid_lines(
    out: &mut Vec<QuadInstance>,
    screen_w: f32,
    screen_h: f32,
    show_subgrid: bool,
    view_origin: (f32, f32),
    zoom: f32,
) {
    let major_color = [0.85, 0.85, 0.95, GRID_LINE_ALPHA];
    let sub_color = [0.90, 0.90, 1.00, SUBGRID_LINE_ALPHA];
    let half_w = screen_w * 0.5;
    let half_h = screen_h * 0.5;
    let half_thickness = (tiles_to_px(GRID_LINE_THICKNESS) * zoom).clamp(1.0, 2.5) * 0.5;
    let sub_half_thickness = half_thickness;
    let px_per_tile = tiles_to_px(1.0) * zoom;
    let view_w_tiles = px_to_tiles(screen_w) / zoom.max(0.001);
    let view_h_tiles = px_to_tiles(screen_h) / zoom.max(0.001);

    let start_col = (view_origin.0 / TILE_GRID).floor() as i32;
    let end_col = ((view_origin.0 + view_w_tiles) / TILE_GRID).ceil() as i32;
    for col in start_col..=end_col {
        let world_x = col as f32 * TILE_GRID;
        let sx = (world_x - view_origin.0) * px_per_tile;
        out.push(QuadInstance {
            center: [sx, half_h],
            half_size: [half_thickness, half_h],
            color: major_color,
        });
    }

    let start_row = (view_origin.1 / TILE_GRID).floor() as i32;
    let end_row = ((view_origin.1 + view_h_tiles) / TILE_GRID).ceil() as i32;
    for row in start_row..=end_row {
        let world_y = row as f32 * TILE_GRID;
        let sy = (world_y - view_origin.1) * px_per_tile;
        out.push(QuadInstance {
            center: [half_w, sy],
            half_size: [half_w, half_thickness],
            color: major_color,
        });
    }

    if show_subgrid {
        let sub_step_world = TILE_GRID / SUBGRID_DIVISIONS as f32;
        let sub_step_px = sub_step_world * px_per_tile;
        if sub_step_px >= 4.0 {
            let start_sub_col = (view_origin.0 / sub_step_world).floor() as i32;
            let end_sub_col = ((view_origin.0 + view_w_tiles) / sub_step_world).ceil() as i32;
            for idx in start_sub_col..=end_sub_col {
                if idx.rem_euclid(SUBGRID_DIVISIONS as i32) == 0 {
                    continue;
                }
                let world_x = idx as f32 * sub_step_world;
                let sx = (world_x - view_origin.0) * px_per_tile;
                out.push(QuadInstance {
                    center: [sx, half_h],
                    half_size: [sub_half_thickness, half_h],
                    color: sub_color,
                });
            }

            let start_sub_row = (view_origin.1 / sub_step_world).floor() as i32;
            let end_sub_row = ((view_origin.1 + view_h_tiles) / sub_step_world).ceil() as i32;
            for idx in start_sub_row..=end_sub_row {
                if idx.rem_euclid(SUBGRID_DIVISIONS as i32) == 0 {
                    continue;
                }
                let world_y = idx as f32 * sub_step_world;
                let sy = (world_y - view_origin.1) * px_per_tile;
                out.push(QuadInstance {
                    center: [half_w, sy],
                    half_size: [half_w, sub_half_thickness],
                    color: sub_color,
                });
            }
        }
    }
}
