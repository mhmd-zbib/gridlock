use engine::render::text::TextSection;

use super::shared::ts;

pub fn main_menu_texts(sw: f32, sh: f32) -> Vec<TextSection> {
    let cx = sw * 0.5;
    let bh = sh * 0.10;
    let gap = sh * 0.05;
    let y_play = sh * 0.28 + bh * 0.26;
    let y_loadout = y_play + bh + gap;
    let y_editor = y_loadout + bh + gap;
    vec![
        ts(
            cx - 160.0,
            sh * 0.12,
            "SHOOTING GAME",
            48.0,
            [1.0, 1.0, 1.0, 1.0],
        ),
        ts(cx - 68.0, y_play, "PLAY GAME", 28.0, [0.0, 0.0, 0.0, 1.0]),
        ts(cx - 62.0, y_loadout, "LOADOUT", 28.0, [0.0, 0.0, 0.0, 1.0]),
        ts(
            cx - 92.0,
            y_editor,
            "LEVEL EDITOR",
            28.0,
            [0.0, 0.0, 0.0, 1.0],
        ),
    ]
}
