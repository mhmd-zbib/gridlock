use engine::render::quad::QuadInstance;

// ---------------------------------------------------------------------------
// Button
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct Button {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: [f32; 4],
    color_hover: [f32; 4],
}

impl Button {
    fn contains(&self, mx: f32, my: f32) -> bool {
        mx >= self.x && mx <= self.x + self.w && my >= self.y && my <= self.y + self.h
    }

    fn instance(&self, mx: f32, my: f32) -> QuadInstance {
        QuadInstance {
            center: [self.x + self.w * 0.5, self.y + self.h * 0.5],
            half_size: [self.w * 0.5, self.h * 0.5],
            color: if self.contains(mx, my) { self.color_hover } else { self.color },
        }
    }
}

// ---------------------------------------------------------------------------
// MainMenu
// ---------------------------------------------------------------------------

pub enum MenuChoice {
    Play,
    Loadout,
    Editor,
}

pub struct MainMenu;

impl MainMenu {
    pub fn new() -> Self {
        Self
    }

    fn buttons(sw: f32, sh: f32) -> [Button; 3] {
        let bw = sw * 0.40;
        let bh = sh * 0.10;
        let cx = sw * 0.5 - bw * 0.5;
        let gap = sh * 0.05;
        let play = Button {
            x: cx, y: sh * 0.28, w: bw, h: bh,
            color:       [0.12, 0.50, 0.12, 1.0],
            color_hover: [0.18, 0.80, 0.18, 1.0],
        };
        let loadout = Button {
            x: cx, y: play.y + bh + gap, w: bw, h: bh,
            color:       [0.08, 0.28, 0.60, 1.0],
            color_hover: [0.14, 0.44, 0.92, 1.0],
        };
        let editor = Button {
            x: cx, y: loadout.y + bh + gap, w: bw, h: bh,
            color:       [0.55, 0.30, 0.05, 1.0],
            color_hover: [0.85, 0.50, 0.10, 1.0],
        };
        [play, loadout, editor]
    }

    pub fn instances(&self, sw: f32, sh: f32, mx: f32, my: f32) -> Vec<QuadInstance> {
        let [play, loadout, editor] = Self::buttons(sw, sh);
        vec![play.instance(mx, my), loadout.instance(mx, my), editor.instance(mx, my)]
    }

    pub fn click(&self, sw: f32, sh: f32, mx: f32, my: f32) -> Option<MenuChoice> {
        let [play, loadout, editor] = Self::buttons(sw, sh);
        if play.contains(mx, my)    { return Some(MenuChoice::Play); }
        if loadout.contains(mx, my) { return Some(MenuChoice::Loadout); }
        if editor.contains(mx, my)  { return Some(MenuChoice::Editor); }
        None
    }
}
