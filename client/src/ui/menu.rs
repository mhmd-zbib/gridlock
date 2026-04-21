use ::ui::components::{PrimaryButton, PrimaryButtonFrame, PrimaryButtonQuad};
use game::render::quad::ShadedQuadInstance;

// ---------------------------------------------------------------------------
// Button
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct Button {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl Button {
    fn contains(&self, mx: f32, my: f32) -> bool {
        mx >= self.x && mx <= self.x + self.w && my >= self.y && my <= self.y + self.h
    }

    fn instance(&self, mx: f32, my: f32) -> ShadedQuadInstance {
        let hovered = self.contains(mx, my);
        let frame = PrimaryButtonFrame::centered(
            [self.x + self.w * 0.5, self.y + self.h * 0.5],
            [self.w, self.h],
            self.w * 0.08,
        );
        let data = PrimaryButton::shaded_instance(frame, hovered);
        quad_from_ui(data)
    }
}

fn quad_from_ui(layer: PrimaryButtonQuad) -> ShadedQuadInstance {
    ShadedQuadInstance {
        center: layer.center,
        half_size: layer.half_size,
        top_color: layer.top_color,
        bottom_color: layer.bottom_color,
        stroke_color: layer.stroke_color,
        accent_color: layer.accent_color,
        params: layer.params,
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
            x: cx,
            y: sh * 0.28,
            w: bw,
            h: bh,
        };
        let loadout = Button {
            x: cx,
            y: play.y + bh + gap,
            w: bw,
            h: bh,
        };
        let editor = Button {
            x: cx,
            y: loadout.y + bh + gap,
            w: bw,
            h: bh,
        };
        [play, loadout, editor]
    }

    pub fn instances(&self, sw: f32, sh: f32, mx: f32, my: f32) -> Vec<ShadedQuadInstance> {
        let [play, loadout, editor] = Self::buttons(sw, sh);
        vec![
            play.instance(mx, my),
            loadout.instance(mx, my),
            editor.instance(mx, my),
        ]
    }

    pub fn click(&self, sw: f32, sh: f32, mx: f32, my: f32) -> Option<MenuChoice> {
        let [play, loadout, editor] = Self::buttons(sw, sh);
        if play.contains(mx, my) {
            return Some(MenuChoice::Play);
        }
        if loadout.contains(mx, my) {
            return Some(MenuChoice::Loadout);
        }
        if editor.contains(mx, my) {
            return Some(MenuChoice::Editor);
        }
        None
    }
}
