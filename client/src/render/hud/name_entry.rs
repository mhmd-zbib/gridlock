use game::render::text::TextSection;

use super::shared::ts;

pub fn name_entry_texts(sw: f32, sh: f32, player_name: &str) -> Vec<TextSection> {
    let display = format!("{}|", player_name);
    vec![
        ts(
            sw * 0.5 - 130.0,
            sh * 0.42,
            "ENTER YOUR NAME",
            28.0,
            [1.0, 1.0, 1.0, 1.0],
        ),
        ts(
            sw * 0.5 - sw * 0.20,
            sh * 0.535,
            display,
            20.0,
            [0.9, 0.9, 0.55, 1.0],
        ),
        ts(
            sw * 0.5 - 145.0,
            sh * 0.65,
            "Press Enter to confirm",
            16.0,
            [0.6, 0.6, 0.6, 1.0],
        ),
        ts(
            sw * 0.5 - 135.0,
            sh * 0.68,
            "Esc: back to main menu",
            14.0,
            [0.45, 0.45, 0.45, 1.0],
        ),
    ]
}
