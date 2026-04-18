use crate::camera::{CameraBehaviorState, TacticalCamera};
use crate::render::views::HudView;
use engine::render::text::TextSection;

use super::shared::{net_status_line, ts};

pub fn play_texts(
    sw: f32,
    _sh: f32,
    view: &HudView<'_>,
    camera: &TacticalCamera,
) -> Vec<TextSection> {
    let ammo_display = view.player.ammo;
    let reloading = view.player.reloading;

    let mut out = vec![
        ts(
            8.0,
            6.0,
            "WASD: move   Shift: sprint   X+WASD: peek   hold click: fire   Esc: menu   F1: editor   F8: debug",
            13.0,
            [0.5, 0.5, 0.5, 1.0],
        ),
        ts(
            8.0,
            22.0,
            format!(
                "{} ({})  ammo: {}/{}{}   R: reload",
                view.player.weapon_name,
                view.player.weapon_class,
                ammo_display,
                view.player.mag_size,
                if reloading { " [reloading]" } else { "" }
            ),
            13.0,
            [0.5, 0.5, 0.5, 1.0],
        ),
        ts(
            8.0,
            38.0,
            view.player.attachments_line.clone(),
            13.0,
            [0.45, 0.45, 0.45, 1.0],
        ),
        ts(
            sw - 170.0,
            6.0,
            "SHOOTING GAME",
            13.0,
            [0.35, 0.35, 0.35, 1.0],
        ),
        ts(
            sw - 210.0,
            20.0,
            net_status_line(view.net),
            12.0,
            [0.35, 0.35, 0.35, 1.0],
        ),
    ];

    if view.enemies.is_empty() && view.player.room_idx.is_none() && view.player.speed == 0.0 {
        // Non-debug mode: enemies vec is empty, no room info, skip debug block.
        return out;
    }

    let px = sw - 310.0;
    let mut py = 28.0;
    let lh = 13.0;

    out.push(ts(
        px,
        py,
        format!(
            "[DEBUG]  spd:{:.2} tiles/s  enemies:{}",
            view.player.speed, view.player.enemy_count,
        ),
        12.0,
        [0.9, 0.9, 0.2, 1.0],
    ));
    py += lh + 2.0;

    let room_info = match view.player.room_idx {
        Some(idx) => format!("Room: {}", idx),
        None => "Room: ---".to_string(),
    };
    out.push(ts(px, py, room_info, 11.0, [0.6, 0.9, 0.6, 1.0]));
    py += lh + 2.0;

    let camera_state = match camera.state() {
        CameraBehaviorState::Combat => "combat",
        CameraBehaviorState::PeekTension => "peek",
        CameraBehaviorState::Exploration => "explore",
    };
    let cam_center = camera.center();
    let cam_offset = camera.offset();
    out.push(ts(
        px,
        py,
        format!(
            "Cam: {}  center({:.2},{:.2})  off({:.2},{:.2})  room:{}  gap:{}",
            camera_state,
            cam_center.0,
            cam_center.1,
            cam_offset.0,
            cam_offset.1,
            if camera.in_room() { "Y" } else { "n" },
            if camera.near_gap() { "Y" } else { "n" }
        ),
        10.0,
        [0.62, 0.78, 0.98, 1.0],
    ));
    py += lh + 2.0;

    for e in &view.enemies {
        if e.is_dummy {
            out.push(ts(
                px,
                py,
                format!("T{} [TARGET] hp:{}", e.idx, e.hp),
                11.0,
                [1.0, 0.85, 0.25, 1.0],
            ));
            py += lh + 3.0;
            continue;
        }

        out.push(ts(
            px,
            py,
            format!(
                "E{} [{}] susp:{:.2} hp:{} vis:{}",
                e.idx,
                e.state_label,
                e.suspicion,
                e.hp,
                if e.in_combat { "Y" } else { "n" }
            ),
            11.0,
            e.color,
        ));
        py += lh;

        out.push(ts(
            px + 8.0,
            py,
            format!(
                "pos:({:.0},{:.0}) anc:({:.0},{:.0})",
                e.pos.0, e.pos.1, e.anchor.0, e.anchor.1
            ),
            10.0,
            [0.6, 0.6, 0.6, 1.0],
        ));
        py += lh;

        out.push(ts(
            px + 8.0,
            py,
            format!("phase:{}", e.phase),
            10.0,
            [0.5, 0.75, 1.0, 1.0],
        ));
        py += lh;

        if let Some(lk) = e.last_known_pos {
            out.push(ts(
                px + 8.0,
                py,
                format!("last_known:({:.0},{:.0})", lk.0, lk.1),
                10.0,
                [0.85, 0.5, 0.85, 1.0],
            ));
            py += lh;
        }

        if let Some(mv) = e.last_move_target {
            out.push(ts(
                px + 8.0,
                py,
                format!("move_to:({:.0},{:.0})", mv.0, mv.1),
                10.0,
                [0.4, 1.0, 0.6, 1.0],
            ));
            py += lh;
        }

        py += 3.0;
    }

    out
}
