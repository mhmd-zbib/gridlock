use serde::{Deserialize, Serialize};

/// An axis-aligned rectangular wall.
/// Used in both the level file (serialised) and the live game (collision).
#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct Wall {
    pub x: f32, // left edge
    pub y: f32, // top edge
    pub w: f32,
    pub h: f32,
    #[serde(default)]
    pub breakable: bool,
    #[serde(default = "default_wall_hp")]
    pub hp: u32,
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
        }
    }

    pub fn new_breakable(x: f32, y: f32, w: f32, h: f32, hp: u32) -> Self {
        Self {
            x,
            y,
            w,
            h,
            breakable: true,
            hp: hp.max(1),
        }
    }

    /// Apply damage to this wall.
    /// Returns true when the wall should be destroyed.
    pub fn take_damage(&mut self, amount: u32) -> bool {
        if !self.breakable {
            return false;
        }
        self.hp = self.hp.saturating_sub(amount.max(1));
        self.hp == 0
    }

    /// True if an AABB centred at (cx, cy) with half-extent `half` overlaps this wall.
    pub fn overlaps(&self, cx: f32, cy: f32, half: f32) -> bool {
        cx + half > self.x
            && cx - half < self.x + self.w
            && cy + half > self.y
            && cy - half < self.y + self.h
    }

    /// Push the centre point (cx, cy) out of this wall along the axis of least penetration.
    /// Does nothing if there is no overlap.
    pub fn push_out(&self, cx: &mut f32, cy: &mut f32, half: f32) {
        if !self.overlaps(*cx, *cy, half) {
            return;
        }

        let pen_right = (*cx + half) - self.x; // how far right edge pokes into left wall face
        let pen_left = (self.x + self.w) - (*cx - half); // how far left edge pokes into right wall face
        let pen_bottom = (*cy + half) - self.y;
        let pen_top = (self.y + self.h) - (*cy - half);

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

    /// True if the point (px, py) is strictly inside this wall.
    /// Used for bullet collision (bullets are treated as points).
    pub fn contains(&self, px: f32, py: f32) -> bool {
        px > self.x && px < self.x + self.w && py > self.y && py < self.y + self.h
    }
}

/// Resolve all wall collisions for one entity centred at (cx, cy) with half-extent `half`.
/// Iterates until no overlap remains (handles corner cases with multiple walls).
pub fn resolve_all(cx: &mut f32, cy: &mut f32, half: f32, walls: &[Wall]) {
    // Two passes is enough for typical level geometry.
    for _ in 0..2 {
        for wall in walls {
            wall.push_out(cx, cy, half);
        }
    }
}
