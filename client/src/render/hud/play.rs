use crate::camera::{CameraBehaviorState, TacticalCamera};
use crate::render::views::HudView;
use game::render::text::TextSection;

use super::shared::{net_status_line, ts};

pub fn play_texts(
    sw: f32,
    sh: f32,
    view: &HudView<'_>,
    camera: &TacticalCamera,
) -> Vec<TextSection> {
    let ammo_display = view.player.ammo;
    let reloading = view.player.reloading;
    let health = view.health;
    let dead = health == 0;

    // Health bar: 20 filled/empty segments.
    let filled = (health as usize * 20 / 100).min(20);
    let bar: String = std::iter::repeat('█')
        .take(filled)
        .chain(std::iter::repeat('░').take(20 - filled))
        .collect();
    let hp_color = if health > 60 {
        [0.2, 1.0, 0.2, 1.0]
    } else if health > 25 {
        [1.0, 0.75, 0.1, 1.0]
    } else {
        [1.0, 0.2, 0.2, 1.0]
    };

    let respawn_timer = view.match_state.map(|m| m.timer).unwrap_or(0);
    let score1 = view.match_state.map(|m| m.score_team1).unwrap_or(0);
    let score2 = view.match_state.map(|m| m.score_team2).unwrap_or(0);

    let mut out = vec![
        // Top-left: controls hint
        ts(
            8.0,
            6.0,
            "WASD: move   Shift: sprint   X+WASD: peek   hold click: fire   Esc: menu   F1: editor   F8: debug",
            13.0,
            [0.5, 0.5, 0.5, 1.0],
        ),
        // Weapon line
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
        // Attachments
        ts(
            8.0,
            38.0,
            view.player.attachments_line.clone(),
            13.0,
            [0.45, 0.45, 0.45, 1.0],
        ),
        // Health bar (bottom-left)
        ts(
            8.0,
            sh - 52.0,
            format!("HP  {} {}/100", bar, health),
            15.0,
            hp_color,
        ),
        // Score (top-center)
        ts(
            sw * 0.5 - 80.0,
            6.0,
            format!("Team 1: {}  —  Team 2: {}", score1, score2),
            16.0,
            [0.9, 0.9, 0.9, 1.0],
        ),
        // Top-right: game title + net
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

    // "SPECTATING" label at the bottom when dead.
    if dead {
        out.push(ts(
            sw * 0.5 - 60.0,
            sh - 26.0,
            "SPECTATING",
            18.0,
            [0.8, 0.8, 0.8, 0.85],
        ));
    }

    // Kill notification — shown for 3 seconds, fades naturally when removed.
    if let Some(killer) = &view.kill_notification {
        out.push(ts(
            sw * 0.5 - 220.0,
            sh * 0.38,
            format!("YOU HAVE BEEN KILLED BY {}", killer),
            24.0,
            [1.0, 0.2, 0.2, 1.0],
        ));
    }

    if respawn_timer > 0 {
        let msg = if dead {
            format!("Respawning in {}s", respawn_timer)
        } else {
            format!("Round over — next round in {}s", respawn_timer)
        };
        out.push(ts(
            sw * 0.5 - 160.0,
            sh * 0.5,
            msg,
            22.0,
            [1.0, 0.3, 0.3, 1.0],
        ));
    }

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
    out.push(ts(
        px,
        py,
        format!(
            "Cam: {}  center({:.2},{:.2})  room:{}  gap:{}",
            camera_state,
            cam_center.0,
            cam_center.1,
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
