use crate::core::world::level::{LevelData, Pos};
use crate::core::world::wall::Wall;
use crate::input::InputState;
use crate::render::quad::QuadInstance;

// ---------------------------------------------------------------------------
// Tool
// ---------------------------------------------------------------------------

#[derive(Default, Clone, Copy, PartialEq)]
pub enum Tool {
    #[default]
    PlayerSpawn, // key 1 — green
    Enemy,       // key 2 — red
    Wall,        // key 3 — brown, drag to draw rectangle
    TargetDummy, // key 4 — yellow
}

// ---------------------------------------------------------------------------
// Editor
// ---------------------------------------------------------------------------

const GRID: f32 = 16.0;
const MIN_WALL: f32 = GRID; // walls smaller than this are discarded

pub struct Editor {
    pub tool: Tool,
    pub level: LevelData,
    pub grid_snap: bool,

    /// Set when left-button is pressed while the Wall tool is active.
    wall_drag_start: Option<(f32, f32)>,

    // Edge detection
    prev_left: bool,
    prev_right: bool,
    prev_f5: bool,
    prev_key_l: bool,
    prev_key_1: bool,
    prev_key_2: bool,
    prev_key_3: bool,
    prev_key_4: bool,
    prev_key_g: bool,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            tool: Tool::default(),
            level: LevelData::default(),
            grid_snap: true,
            wall_drag_start: None,
            prev_left: false,
            prev_right: false,
            prev_f5: false,
            prev_key_l: false,
            prev_key_1: false,
            prev_key_2: false,
            prev_key_3: false,
            prev_key_4: false,
            prev_key_g: false,
        }
    }

    pub fn update(&mut self, input: &InputState) {
        let raw_mx = input.mouse_x as f32;
        let raw_my = input.mouse_y as f32;
        let mx = if self.grid_snap {
            snap(raw_mx, GRID)
        } else {
            raw_mx
        };
        let my = if self.grid_snap {
            snap(raw_my, GRID)
        } else {
            raw_my
        };

        let just_pressed = input.mouse_left && !self.prev_left;
        let just_released = !input.mouse_left && self.prev_left;
        let just_right_pressed = input.mouse_right && !self.prev_right;

        // --- tool selection ---
        if input.key_1 && !self.prev_key_1 {
            self.tool = Tool::PlayerSpawn;
            self.wall_drag_start = None;
            println!("[editor] tool → player spawn  (1)");
        }
        if input.key_2 && !self.prev_key_2 {
            self.tool = Tool::Enemy;
            self.wall_drag_start = None;
            println!("[editor] tool → enemy  (2)");
        }
        if input.key_3 && !self.prev_key_3 {
            self.tool = Tool::Wall;
            println!("[editor] tool → wall  (3) — drag to draw, right-click to delete");
        }
        if input.key_4 && !self.prev_key_4 {
            self.tool = Tool::TargetDummy;
            self.wall_drag_start = None;
            println!("[editor] tool → target dummy  (4)");
        }

        // --- grid snap toggle ---
        if input.key_g && !self.prev_key_g {
            self.grid_snap = !self.grid_snap;
            println!(
                "[editor] grid snap {}",
                if self.grid_snap { "ON" } else { "OFF" }
            );
        }

        // --- place / drag ---
        match self.tool {
            Tool::Wall => {
                if just_pressed {
                    self.wall_drag_start = Some((mx, my));
                }
                if just_released {
                    if let Some(start) = self.wall_drag_start.take() {
                        let x = start.0.min(mx);
                        let y = start.1.min(my);
                        let w = (start.0 - mx).abs();
                        let h = (start.1 - my).abs();
                        if w >= MIN_WALL && h >= MIN_WALL {
                            self.level.walls.push(Wall::new(x, y, w, h));
                            println!(
                                "[editor] wall placed  ({x:.0}, {y:.0})  {w:.0}×{h:.0}  total={}",
                                self.level.walls.len()
                            );
                        }
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
            self.delete_at(raw_mx, raw_my);
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
        self.prev_key_g = input.key_g;
    }

    fn place(&mut self, x: f32, y: f32) {
        match self.tool {
            Tool::PlayerSpawn => {
                self.level.player_spawn = Some(Pos { x, y });
                println!("[editor] player spawn → ({x:.0}, {y:.0})");
            }
            Tool::Enemy => {
                self.level.enemies.push(Pos { x, y });
                println!(
                    "[editor] enemy ({x:.0}, {y:.0})  total={}",
                    self.level.enemies.len()
                );
            }
            Tool::TargetDummy => {
                self.level.target_enemies.push(Pos { x, y });
                println!(
                    "[editor] target dummy ({x:.0}, {y:.0})  total={}",
                    self.level.target_enemies.len()
                );
            }
            Tool::Wall => {} // handled via drag
        }
    }

    fn delete_at(&mut self, x: f32, y: f32) {
        // Walls — delete if the click point is inside the rect.
        if let Some(idx) = self.level.walls.iter().position(|w| w.contains(x, y)) {
            let w = self.level.walls.remove(idx);
            println!(
                "[editor] wall removed  ({:.0}, {:.0}) {:.0}×{:.0}  remaining={}",
                w.x,
                w.y,
                w.w,
                w.h,
                self.level.walls.len()
            );
            return;
        }

        // Enemies — delete nearest within radius.
        const R: f32 = 20.0;
        if let Some(idx) = nearest_idx(&self.level.enemies, x, y, R) {
            self.level.enemies.remove(idx);
            println!(
                "[editor] enemy removed  remaining={}",
                self.level.enemies.len()
            );
            return;
        }

        if let Some(idx) = nearest_idx(&self.level.target_enemies, x, y, R) {
            self.level.target_enemies.remove(idx);
            println!(
                "[editor] target dummy removed  remaining={}",
                self.level.target_enemies.len()
            );
            return;
        }

        // Player spawn — delete if near enough.
        if let Some(sp) = self.level.player_spawn {
            if dist(sp.x, sp.y, x, y) < R {
                self.level.player_spawn = None;
                println!("[editor] player spawn cleared");
            }
        }
    }

    pub fn save(&self, path: &str) {
        match self.level.save(path) {
            Ok(_) => println!("[editor] saved   → {path}"),
            Err(e) => println!("[editor] save error: {e}"),
        }
    }

    pub fn load(&mut self, path: &str) {
        match LevelData::load(path) {
            Ok(data) => {
                println!(
                    "[editor] loaded  ← {path}  enemies={}  targets={}  walls={}",
                    data.enemies.len(),
                    data.target_enemies.len(),
                    data.walls.len()
                );
                self.level = data;
            }
            Err(e) => println!("[editor] load error: {e}"),
        }
    }

    /// Build the instance list for the editor overlay.
    pub fn instances(&self, mouse_x: f32, mouse_y: f32) -> Vec<QuadInstance> {
        let mx = if self.grid_snap {
            snap(mouse_x, GRID)
        } else {
            mouse_x
        };
        let my = if self.grid_snap {
            snap(mouse_y, GRID)
        } else {
            mouse_y
        };

        let mut out = Vec::new();

        // Walls — brownish gray
        for w in &self.level.walls {
            out.push(QuadInstance {
                center: [w.x + w.w * 0.5, w.y + w.h * 0.5],
                half_size: [w.w * 0.5, w.h * 0.5],
                color: [0.45, 0.4, 0.35, 1.0],
            });
        }

        // Player spawn — green
        if let Some(sp) = self.level.player_spawn {
            out.push(QuadInstance {
                center: [sp.x, sp.y],
                half_size: [10.0, 10.0],
                color: [0.2, 1.0, 0.2, 1.0],
            });
        }

        // Enemies — red
        for e in &self.level.enemies {
            out.push(QuadInstance {
                center: [e.x, e.y],
                half_size: [8.0, 8.0],
                color: [1.0, 0.2, 0.2, 1.0],
            });
        }

        // Target dummies — yellow
        for e in &self.level.target_enemies {
            out.push(QuadInstance {
                center: [e.x, e.y],
                half_size: [8.0, 8.0],
                color: [1.0, 0.85, 0.2, 1.0],
            });
        }

        // Ghost: drag preview (wall tool) or cursor dot (other tools)
        match self.tool {
            Tool::Wall => {
                if let Some(start) = self.wall_drag_start {
                    // Live preview rectangle while dragging.
                    let x = start.0.min(mx);
                    let y = start.1.min(my);
                    let w = (start.0 - mx).abs().max(1.0);
                    let h = (start.1 - my).abs().max(1.0);
                    out.push(QuadInstance {
                        center: [x + w * 0.5, y + h * 0.5],
                        half_size: [w * 0.5, h * 0.5],
                        color: [0.45, 0.4, 0.35, 0.45],
                    });
                } else {
                    // Small cursor dot showing snap position.
                    out.push(QuadInstance {
                        center: [mx, my],
                        half_size: [3.0, 3.0],
                        color: [0.45, 0.4, 0.35, 0.5],
                    });
                }
            }
            Tool::PlayerSpawn => {
                out.push(QuadInstance {
                    center: [mx, my],
                    half_size: [10.0, 10.0],
                    color: [0.2, 1.0, 0.2, 0.35],
                });
            }
            Tool::Enemy => {
                out.push(QuadInstance {
                    center: [mx, my],
                    half_size: [8.0, 8.0],
                    color: [1.0, 0.2, 0.2, 0.35],
                });
            }
            Tool::TargetDummy => {
                out.push(QuadInstance {
                    center: [mx, my],
                    half_size: [8.0, 8.0],
                    color: [1.0, 0.85, 0.2, 0.35],
                });
            }
        }

        out
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn snap(v: f32, grid: f32) -> f32 {
    (v / grid).round() * grid
}

fn dist(ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let dx = ax - bx;
    let dy = ay - by;
    (dx * dx + dy * dy).sqrt()
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
