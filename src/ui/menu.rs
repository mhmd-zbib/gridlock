use crate::render::quad::QuadInstance;

// ---------------------------------------------------------------------------
// Button
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct Button {
    x: f32, y: f32, w: f32, h: f32,
    color:       [f32; 4],
    color_hover: [f32; 4],
}

impl Button {
    fn contains(&self, mx: f32, my: f32) -> bool {
        mx >= self.x && mx <= self.x + self.w && my >= self.y && my <= self.y + self.h
    }

    fn instance(&self, mx: f32, my: f32) -> QuadInstance {
        QuadInstance {
            center:    [self.x + self.w * 0.5, self.y + self.h * 0.5],
            half_size: [self.w * 0.5, self.h * 0.5],
            color:     if self.contains(mx, my) { self.color_hover } else { self.color },
        }
    }
}

// ---------------------------------------------------------------------------
// MainMenu
// ---------------------------------------------------------------------------

pub enum MenuChoice { Play, Editor }

/// Two large coloured buttons, laid out relative to the window size.
///
/// Green  (top)    → Play
/// Orange (bottom) → Level Editor
pub struct MainMenu;

impl MainMenu {
    pub fn new() -> Self {
        println!("[menu] MAIN MENU");
        println!("[menu]   green  → PLAY GAME");
        println!("[menu]   orange → LEVEL EDITOR");
        println!("[menu]   click a button to continue");
        Self
    }

    fn buttons(sw: f32, sh: f32) -> (Button, Button) {
        let bw = sw * 0.40;
        let bh = sh * 0.12;
        let cx = sw * 0.5 - bw * 0.5;
        let play = Button {
            x: cx, y: sh * 0.35, w: bw, h: bh,
            color:       [0.12, 0.50, 0.12, 1.0],
            color_hover: [0.18, 0.80, 0.18, 1.0],
        };
        let editor = Button {
            x: cx, y: sh * 0.55, w: bw, h: bh,
            color:       [0.55, 0.30, 0.05, 1.0],
            color_hover: [0.85, 0.50, 0.10, 1.0],
        };
        (play, editor)
    }

    pub fn instances(&self, sw: f32, sh: f32, mx: f32, my: f32) -> Vec<QuadInstance> {
        let (play, editor) = Self::buttons(sw, sh);
        vec![play.instance(mx, my), editor.instance(mx, my)]
    }

    /// Returns the chosen action if the click lands on a button.
    pub fn click(&self, sw: f32, sh: f32, mx: f32, my: f32) -> Option<MenuChoice> {
        let (play, editor) = Self::buttons(sw, sh);
        if play.contains(mx, my)   { return Some(MenuChoice::Play);   }
        if editor.contains(mx, my) { return Some(MenuChoice::Editor); }
        None
    }
}
