use serde::{Deserialize, Serialize};

/// How long (in tiles) each breakable wall segment is along the split axis.
const SEGMENT_SIZE: f32 = 0.25;

/// An axis-aligned rectangular wall.
/// Used in both the level file (serialised) and the live game (collision).
#[derive(Serialize, Deserialize, Clone)]
pub struct Wall {
    pub x: f32, // left edge
    pub y: f32, // top edge
    pub w: f32,
    pub h: f32,
    #[serde(default)]
    pub breakable: bool,
    #[serde(default = "default_wall_hp")]
    pub hp: u32,
    /// Axis of the edge segment: Some(true) = horizontal, Some(false) = vertical, None = corner/other.
    /// Written for editor-placed walls so the axis survives a JSON round-trip.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub horizontal: Option<bool>,
    /// Per-segment alive flags.  Only populated for breakable walls at runtime.
    /// Skipped during serialisation — regenerated from dimensions on load.
    #[serde(skip)]
    pub segments: Vec<bool>,
    /// True for walls that were synthesised from collidable props at load time.
    /// These walls exist for collision only; the prop renderer owns their visual.
    #[serde(skip)]
    pub is_prop: bool,
}

const fn default_wall_hp() -> u32 {
    1
}

impl Wall {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            x,
            y,
            w,
            h,
            breakable: false,
            hp: default_wall_hp(),
            horizontal: None,
            segments: Vec::new(),
            is_prop: false,
        }
    }

    pub fn new_breakable(x: f32, y: f32, w: f32, h: f32, hp: u32) -> Self {
        let mut wall = Self {
            x,
            y,
            w,
            h,
            breakable: true,
            hp: hp.max(1),
            horizontal: None,
            segments: Vec::new(),
            is_prop: false,
        };
        wall.init_segments();
        wall
    }

    /// Compute per-segment alive flags based on the wall's dimensions.
    /// Segments are arranged along the longer axis in steps of `SEGMENT_SIZE`.
    /// Non-breakable walls are left with an empty segment list.
    pub fn init_segments(&mut self) {
        if !self.breakable {
            self.segments.clear();
            return;
        }
        let longer = self.w.max(self.h);
        let count = ((longer / SEGMENT_SIZE).floor() as usize).max(1);
        self.segments = vec![true; count];
    }

    /// Returns the AABB `(x, y, w, h)` of segment `i` out of `n` total segments.
    pub fn segment_rect(&self, i: usize, n: usize) -> (f32, f32, f32, f32) {
        let nf = n as f32;
        let fi = i as f32;
        if self.w >= self.h {
            let sw = self.w / nf;
            (self.x + fi * sw, self.y, sw, self.h)
        } else {
            let sh = self.h / nf;
            (self.x, self.y + fi * sh, self.w, sh)
        }
    }

    /// Apply damage at position `(px, py)`.
    /// For breakable walls with segments the hit segment is destroyed.
    /// Returns `true` when the wall should be removed entirely.
    pub fn take_damage_at(&mut self, px: f32, py: f32, amount: u32) -> bool {
        if !self.breakable {
            return false;
        }
        if self.segments.is_empty() {
            // Fallback for walls whose segments were never initialised.
            self.hp = self.hp.saturating_sub(amount.max(1));
            return self.hp == 0;
        }
        let n = self.segments.len();
        for i in 0..n {
            if !self.segments[i] {
                continue;
            }
            let (sx, sy, sw, sh) = self.segment_rect(i, n);
            if px > sx && px < sx + sw && py > sy && py < sy + sh {
                self.segments[i] = false;
                break;
            }
        }
        self.segments.iter().all(|&alive| !alive)
    }

    /// True if an AABB centred at (cx, cy) with half-extent `half` overlaps this wall.
    pub fn overlaps(&self, cx: f32, cy: f32, half: f32) -> bool {
        cx + half > self.x
            && cx - half < self.x + self.w
            && cy + half > self.y
            && cy - half < self.y + self.h
    }

    /// Push the centre point (cx, cy) out of this wall.
    /// For breakable walls only pushes out of alive segments.
    pub fn push_out(&self, cx: &mut f32, cy: &mut f32, half: f32) {
        if !self.breakable || self.segments.is_empty() {
            if self.overlaps(*cx, *cy, half) {
                push_out_of_aabb(cx, cy, half, self.x, self.y, self.w, self.h);
            }
            return;
        }
        let n = self.segments.len();
        for i in 0..n {
            if !self.segments[i] {
                continue;
            }
            let (sx, sy, sw, sh) = self.segment_rect(i, n);
            if *cx + half > sx && *cx - half < sx + sw && *cy + half > sy && *cy - half < sy + sh {
                push_out_of_aabb(cx, cy, half, sx, sy, sw, sh);
            }
        }
    }

    /// True if the point (px, py) is inside this wall (or any alive segment).
    /// Used for bullet collision — bullets are treated as points.
    pub fn contains(&self, px: f32, py: f32) -> bool {
        if !self.breakable || self.segments.is_empty() {
            return px > self.x && px < self.x + self.w && py > self.y && py < self.y + self.h;
        }
        let n = self.segments.len();
        for i in 0..n {
            if !self.segments[i] {
                continue;
            }
            let (sx, sy, sw, sh) = self.segment_rect(i, n);
            if px > sx && px < sx + sw && py > sy && py < sy + sh {
                return true;
            }
        }
        false
    }
}

// ---------------------------------------------------------------------------
// Collision helpers
// ---------------------------------------------------------------------------

/// Resolve penetration between a circle (cx, cy, half) and an AABB (x, y, w, h).
/// Pushes along the axis of minimum penetration depth.
fn push_out_of_aabb(cx: &mut f32, cy: &mut f32, half: f32, x: f32, y: f32, w: f32, h: f32) {
    let pen_right = (*cx + half) - x;
    let pen_left = (x + w) - (*cx - half);
    let pen_bottom = (*cy + half) - y;
    let pen_top = (y + h) - (*cy - half);
    let min = pen_right.min(pen_left).min(pen_bottom).min(pen_top);
    if min == pen_right {
        *cx -= pen_right;
    } else if min == pen_left {
        *cx += pen_left;
    } else if min == pen_bottom {
        *cy -= pen_bottom;
    } else {
        *cy += pen_top;
    }
}

// ---------------------------------------------------------------------------
// Breakable-chain piercing
// ---------------------------------------------------------------------------

/// When a bullet enters a breakable wall, this destroys every consecutive
/// breakable wall along `dir` and returns the world position where the chain
/// ends (the exit edge of the last broken wall).
///
/// Pattern: air → [breakable]+ → air  ⟹  all breakables destroyed, bullet
/// dies at the exit of the last one.  A solid wall immediately after the chain
/// also terminates it (bullet stops at the chain exit, not inside the solid).
pub fn pierce_breakable_chain(
    walls: &mut Vec<Wall>,
    entry: (f32, f32),
    dir: (f32, f32),
    damage: u32,
) -> (f32, f32) {
    let mut pos = entry;
    loop {
        let Some(idx) = walls.iter().position(|w| w.breakable && w.contains(pos.0, pos.1)) else {
            return pos;
        };

        let exit = aabb_exit_point(&walls[idx], pos, dir);

        let destroyed = walls[idx].take_damage_at(pos.0, pos.1, damage);
        if destroyed {
            walls.remove(idx);
        }

        // Probe one step past the exit to classify the next cell.
        const PROBE: f32 = 0.001;
        let next = (exit.0 + dir.0 * PROBE, exit.1 + dir.1 * PROBE);

        // Solid wall immediately after → stop here (don't enter solid).
        if walls.iter().any(|w| !w.breakable && w.contains(next.0, next.1)) {
            return exit;
        }

        // Another breakable → keep chaining.
        if walls.iter().any(|w| w.breakable && w.contains(next.0, next.1)) {
            pos = next;
            continue;
        }

        // Air → bullet dies at exit of this last breakable.
        return exit;
    }
}

/// Returns the world-space point where a ray travelling through a wall AABB exits it.
fn aabb_exit_point(wall: &Wall, origin: (f32, f32), dir: (f32, f32)) -> (f32, f32) {
    let inv_x = if dir.0.abs() > 1e-9 { 1.0 / dir.0 } else { f32::INFINITY };
    let inv_y = if dir.1.abs() > 1e-9 { 1.0 / dir.1 } else { f32::INFINITY };

    let tx1 = (wall.x - origin.0) * inv_x;
    let tx2 = (wall.x + wall.w - origin.0) * inv_x;
    let ty1 = (wall.y - origin.1) * inv_y;
    let ty2 = (wall.y + wall.h - origin.1) * inv_y;

    let tmax = tx1.max(tx2).min(ty1.max(ty2));
    (origin.0 + dir.0 * tmax, origin.1 + dir.1 * tmax)
}

/// Resolve all wall collisions for one entity centred at (cx, cy) with half-extent `half`.
/// Iterates until no overlap remains (handles corner cases with multiple walls).
pub fn resolve_all(cx: &mut f32, cy: &mut f32, half: f32, walls: &[Wall]) {
    // Two passes is enough for typical level geometry.
    // Use an expanded bounding box to quickly reject distant walls before the
    // full push_out calculation — critical for large maps with many walls.
    let search = half + 0.5; // small buffer to catch walls just outside half-extent
    for _ in 0..2 {
        for wall in walls {
            if *cx + search < wall.x
                || *cx - search > wall.x + wall.w
                || *cy + search < wall.y
                || *cy - search > wall.y + wall.h
            {
                continue;
            }
            wall.push_out(cx, cy, half);
        }
    }
}
