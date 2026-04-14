use engine::render::quad::QuadInstance;

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

    fn instance(&self, mx: f32, my: f32, enabled: bool) -> QuadInstance {
        let color = if !enabled {
            [0.25, 0.25, 0.25, 1.0]
        } else if self.contains(mx, my) {
            self.color_hover
        } else {
            self.color
        };
        QuadInstance {
            center: [self.x + self.w * 0.5, self.y + self.h * 0.5],
            half_size: [self.w * 0.5, self.h * 0.5],
            color,
        }
    }
}

pub enum LobbyChoice {
    Team1,
    Team2,
    StartGame,
}

pub struct LobbyMenu;

impl LobbyMenu {
    pub fn new() -> Self {
        Self
    }

    fn team1_button(sw: f32, sh: f32, selected_team: u8) -> Button {
        let bw = sw * 0.34;
        let bh = sh * 0.10;
        let x = sw * 0.5 - bw - sw * 0.03;
        let selected = selected_team == 1;
        Button {
            x,
            y: sh * 0.44,
            w: bw,
            h: bh,
            color: if selected {
                [0.18, 0.62, 0.22, 1.0]
            } else {
                [0.10, 0.38, 0.12, 1.0]
            },
            color_hover: [0.22, 0.78, 0.28, 1.0],
        }
    }

    fn team2_button(sw: f32, sh: f32, selected_team: u8) -> Button {
        let bw = sw * 0.34;
        let bh = sh * 0.10;
        let x = sw * 0.5 + sw * 0.03;
        let selected = selected_team == 2;
        Button {
            x,
            y: sh * 0.44,
            w: bw,
            h: bh,
            color: if selected {
                [0.20, 0.30, 0.74, 1.0]
            } else {
                [0.11, 0.18, 0.48, 1.0]
            },
            color_hover: [0.28, 0.40, 0.92, 1.0],
        }
    }

    fn start_button(sw: f32, sh: f32) -> Button {
        let bw = sw * 0.40;
        let bh = sh * 0.10;
        Button {
            x: sw * 0.5 - bw * 0.5,
            y: sh * 0.62,
            w: bw,
            h: bh,
            color: [0.55, 0.34, 0.06, 1.0],
            color_hover: [0.82, 0.50, 0.09, 1.0],
        }
    }

    pub fn instances(
        &self,
        sw: f32,
        sh: f32,
        mx: f32,
        my: f32,
        selected_team: u8,
        can_start: bool,
    ) -> Vec<QuadInstance> {
        let team1 = Self::team1_button(sw, sh, selected_team);
        let team2 = Self::team2_button(sw, sh, selected_team);
        let start = Self::start_button(sw, sh);
        vec![
            QuadInstance {
                center: [sw * 0.5, sh * 0.5],
                half_size: [sw * 0.45, sh * 0.36],
                color: [0.06, 0.06, 0.08, 0.95],
            },
            team1.instance(mx, my, true),
            team2.instance(mx, my, true),
            start.instance(mx, my, can_start),
        ]
    }

    pub fn click(
        &self,
        sw: f32,
        sh: f32,
        mx: f32,
        my: f32,
        can_start: bool,
    ) -> Option<LobbyChoice> {
        let team1 = Self::team1_button(sw, sh, 0);
        let team2 = Self::team2_button(sw, sh, 0);
        let start = Self::start_button(sw, sh);
        if team1.contains(mx, my) {
            return Some(LobbyChoice::Team1);
        }
        if team2.contains(mx, my) {
            return Some(LobbyChoice::Team2);
        }
        if can_start && start.contains(mx, my) {
            return Some(LobbyChoice::StartGame);
        }
        None
    }
}
