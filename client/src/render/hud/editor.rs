use crate::ui::editor::{Editor, Tool};
use game::render::text::TextSection;

use super::shared::ts;

pub fn editor_texts(sw: f32, sh: f32, editor: &Editor) -> Vec<TextSection> {
    let tool = match editor.tool {
        Tool::PlayerSpawn => "Player Spawn",
        Tool::Enemy => "Enemy",
        Tool::Wall => "Wall (2-point, 0.1 tile)",
        Tool::TargetDummy => "Target Dummy",
        Tool::Breakable => "Breakable Wall (2-point)",
        Tool::Prop => "Prop",
        Tool::BaseMap => "Base Map (2-point bounds)",
        Tool::Team1Spawn => "Team 1 Spawn (blue)",
        Tool::Team2Spawn => "Team 2 Spawn (orange)",
    };
    let prop_info = match editor.selected_prop_asset() {
        Some(asset) => format!(
            "Prop Id: {} ({}/{})  {:.2}x{:.2}  collider:{}",
            asset.id,
            editor.selected_prop_asset_index() + 1,
            editor.prop_assets().len(),
            asset.width,
            asset.height,
            if asset.is_collider { "yes" } else { "no" }
        ),
        None => "Prop Id: (none found in assets/props/*.json)".to_string(),
    };
    let assets_line = if editor.prop_assets().is_empty() {
        "Prop Ids: (none)".to_string()
    } else {
        let mut labels: Vec<String> = editor
            .prop_assets()
            .iter()
            .enumerate()
            .take(6)
            .map(|(idx, asset)| {
                if idx == editor.selected_prop_asset_index() {
                    format!("[{}]", asset.id)
                } else {
                    asset.id.clone()
                }
            })
            .collect();
        if editor.prop_assets().len() > 6 {
            labels.push(format!("+{}", editor.prop_assets().len() - 6));
        }
        format!("Prop Ids: {}", labels.join("  "))
    };
    let grid = format!(
        "Snap: {}  Inner Grid: {}",
        editor.active_snap_label(),
        if editor.show_subgrid { "ON" } else { "OFF" }
    );
    let breakables = editor.level.walls.iter().filter(|w| w.breakable).count();
    let solids = editor.level.walls.len().saturating_sub(breakables);
    let stats = format!(
        "Enemies: {}  Targets: {}  Walls: {}  Breakables: {}  Props: {}  Map: {}  Zoom: {:.2}x",
        editor.level.enemies.len(),
        editor.level.target_enemies.len(),
        solids,
        breakables,
        editor.level.props.len(),
        match editor.level.map_bounds {
            Some(b) => format!("{:.1}x{:.1}", b.w, b.h),
            None => "--".to_string(),
        },
        editor.zoom
    );
    vec![
        ts(8.0, 6.0, "LEVEL EDITOR", 18.0, [1.0, 0.7, 0.2, 1.0]),
        ts(8.0, 28.0, tool, 15.0, [1.0, 1.0, 1.0, 1.0]),
        ts(8.0, 46.0, grid, 13.0, [0.6, 0.6, 0.6, 1.0]),
        ts(8.0, 64.0, prop_info, 13.0, [0.6, 0.7, 0.9, 1.0]),
        ts(8.0, 82.0, assets_line, 13.0, [0.58, 0.65, 0.86, 1.0]),
        ts(
            6.0,
            sh - 38.0,
            "1: Spawn   2: Enemy   3: Wall   4: Target   5: Breakable   6: Prop   7: Base Map   8: Team1 Spawn   9: Team2 Spawn   Q/E: Prop Id",
            13.0,
            [0.55, 0.55, 0.55, 1.0],
        ),
        ts(
            6.0,
            sh - 20.0,
            "Left: place   Right: delete   Wheel or +/-: zoom   WASD/Arrows: pan   F5: save   L: load   F1: play   Esc: menu",
            13.0,
            [0.55, 0.55, 0.55, 1.0],
        ),
        ts(sw - 180.0, 6.0, stats, 13.0, [0.5, 0.5, 0.5, 1.0]),
    ]
}
