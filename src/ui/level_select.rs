use std::path::PathBuf;

use crate::core::world::level::LevelData;
use crate::render::quad::QuadInstance;

pub struct LevelEntry {
    pub path: PathBuf,
    pub id: String,
}

pub struct LevelSelect {
    pub levels: Vec<LevelEntry>,
    pub selected: usize,

    // Edge-detection for keyboard navigation.
    prev_up: bool,
    prev_down: bool,
}

impl LevelSelect {
    pub fn new() -> Self {
        let levels = scan();
        let s = Self {
            levels,
            selected: 0,
            prev_up: false,
            prev_down: false,
        };
        s.print_list();
        s
    }

    /// Re-scan the levels directory (call when entering this screen).
    pub fn refresh(&mut self) {
        self.levels = scan();
        self.selected = 0;
        self.print_list();
    }

    /// Drive keyboard navigation. Takes raw `up` / `down` bools from InputState.
    /// Returns `true` on the first frame a direction is pressed (just-pressed).
    pub fn navigate(&mut self, up: bool, down: bool) {
        if up && !self.prev_up && !self.levels.is_empty() {
            if self.selected == 0 {
                self.selected = self.levels.len() - 1;
            } else {
                self.selected -= 1;
            }
            println!(
                "[select] ↑  [{}] {} ({})",
                self.selected,
                self.levels[self.selected].id,
                self.levels[self.selected].path.display()
            );
        }
        if down && !self.prev_down && !self.levels.is_empty() {
            self.selected = (self.selected + 1) % self.levels.len();
            println!(
                "[select] ↓  [{}] {} ({})",
                self.selected,
                self.levels[self.selected].id,
                self.levels[self.selected].path.display()
            );
        }
        self.prev_up = up;
        self.prev_down = down;
    }

    pub fn selected_path(&self) -> Option<&str> {
        self.levels.get(self.selected)?.path.to_str()
    }

    /// Stacked row of level rectangles.
    /// Selected = bright white, others = dim.
    pub fn instances(&self, sw: f32, sh: f32) -> Vec<QuadInstance> {
        let mut out = Vec::new();

        if self.levels.is_empty() {
            // Red "no levels" indicator centred on screen.
            out.push(QuadInstance {
                center: [sw * 0.5, sh * 0.5],
                half_size: [90.0, 22.0],
                color: [0.5, 0.08, 0.08, 1.0],
            });
            return out;
        }

        let bw = sw * 0.55;
        let bh = (sh * 0.60 / self.levels.len() as f32).min(52.0);
        let gap = 6.0;
        let cx = sw * 0.5 - bw * 0.5;
        let start_y = sh * 0.20;

        for (i, _) in self.levels.iter().enumerate() {
            let cy = start_y + i as f32 * (bh + gap) + bh * 0.5;
            out.push(QuadInstance {
                center: [cx + bw * 0.5, cy],
                half_size: [bw * 0.5, bh * 0.5],
                color: if i == self.selected {
                    [0.90, 0.90, 0.90, 1.0]
                } else {
                    [0.22, 0.22, 0.22, 1.0]
                },
            });
        }

        // Small arrow indicator on the left of the selected row.
        if let Some(i) = self.levels.get(self.selected).map(|_| self.selected) {
            let cy = start_y + i as f32 * (bh + gap) + bh * 0.5;
            out.push(QuadInstance {
                center: [cx - 14.0, cy],
                half_size: [5.0, 5.0],
                color: [1.0, 1.0, 0.0, 1.0],
            });
        }

        out
    }

    fn print_list(&self) {
        println!("[select] SELECT LEVEL  (↑/↓ navigate · Enter confirm · Esc back)");
        if self.levels.is_empty() {
            println!("[select]   (no .json files found in levels/ — create one in the editor)");
        }
        for (i, p) in self.levels.iter().enumerate() {
            let mark = if i == self.selected { "→" } else { " " };
            println!("[select]   {mark} [{}] {} ({})", i, p.id, p.path.display());
        }
    }
}

fn scan() -> Vec<LevelEntry> {
    let Ok(dir) = std::fs::read_dir("levels") else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = dir
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("json"))
        .collect();
    paths.sort();

    let mut levels = Vec::new();
    for path in paths {
        let Some(path_str) = path.to_str() else {
            continue;
        };
        match LevelData::load(path_str) {
            Ok(level) => levels.push(LevelEntry { path, id: level.id }),
            Err(e) => println!("[select] skipping '{}' ({e})", path.display()),
        }
    }
    levels
}
