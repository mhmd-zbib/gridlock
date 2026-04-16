use crate::ui::loadout::LoadoutMenu;
use engine::render::text::TextSection;
use game::entity::weapon::attachment::AttachmentCategory;

use super::shared::ts;

pub fn loadout_texts(sw: f32, sh: f32, loadout: &LoadoutMenu) -> Vec<TextSection> {
    let mut out = vec![
        ts(
            sw * 0.5 - 145.0,
            sh * 0.10,
            "LOADOUT BUILDER",
            42.0,
            [1.0, 1.0, 1.0, 1.0],
        ),
        ts(
            sw * 0.5 - 220.0,
            sh * 0.88,
            "Up/Down: row   Left/Right: option   Enter/Esc: back",
            15.0,
            [0.55, 0.55, 0.55, 1.0],
        ),
    ];

    let bw = sw * 0.64;
    let bh = (sh * 0.55 / 7.0).min(48.0);
    let gap = 8.0;
    let start_y = sh * 0.24;
    let text_x = sw * 0.5 - bw * 0.5 + 16.0;

    let weapon_line = format!(
        "Weapon: {} ({})",
        loadout.selected_weapon_name(),
        loadout.selected_weapon_class_label()
    );
    out.push(ts(
        text_x,
        start_y + (bh - 18.0) * 0.5,
        weapon_line,
        20.0,
        if loadout.selected_row() == 0 {
            [0.05, 0.05, 0.05, 1.0]
        } else {
            [0.82, 0.82, 0.82, 1.0]
        },
    ));

    for (idx, category) in AttachmentCategory::all().iter().enumerate() {
        let row = idx + 1;
        let row_y = start_y + row as f32 * (bh + gap);
        let supported = loadout.selected_weapon_supports(*category);
        let line = format!(
            "{}: {}",
            category.label(),
            loadout.selected_attachment_name(*category)
        );
        out.push(ts(
            text_x,
            row_y + (bh - 18.0) * 0.5,
            line,
            20.0,
            if !supported {
                [0.45, 0.18, 0.18, 1.0]
            } else if loadout.selected_row() == row {
                [0.05, 0.05, 0.05, 1.0]
            } else {
                [0.82, 0.82, 0.82, 1.0]
            },
        ));
    }

    out
}
