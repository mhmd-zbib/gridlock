use crate::net::NetClient;
use engine::render::text::TextSection;
use game::ai::awareness::AiState;
use game::entity::enemy::EnemyKind;
use game::entity::weapon::attachment::AttachmentCategory;
use game::game::Game;
use game::world::camera::{CameraBehaviorState, TacticalCamera};
use net::SelfState;

use super::shared::{net_status_line, ts};

pub fn play_texts(
    sw: f32,
    _sh: f32,
    game: &Game,
    camera: &TacticalCamera,
    debug: bool,
    net: Option<&NetClient>,
    server_me: Option<&SelfState>,
) -> Vec<TextSection> {
    let attachments_line = AttachmentCategory::all()
        .iter()
        .map(|category| {
            format!(
                "{}={}",
                category.label(),
                game.player.attachment_name_for(*category).unwrap_or("-")
            )
        })
        .collect::<Vec<_>>()
        .join("  ");

    let ammo_display = server_me
        .map(|me| me.ammo)
        .unwrap_or(game.player.ammo_in_mag() as u8);
    let reloading = server_me
        .map(|me| me.reload_progress > 0)
        .unwrap_or(game.player.is_reloading());

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
                game.player.weapon_name(),
                game.player.weapon_class_label(),
                ammo_display,
                game.player.mag_size(),
                if reloading { " [reloading]" } else { "" }
            ),
            13.0,
            [0.5, 0.5, 0.5, 1.0],
        ),
        ts(8.0, 38.0, attachments_line, 13.0, [0.45, 0.45, 0.45, 1.0]),
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
            net_status_line(net),
            12.0,
            [0.35, 0.35, 0.35, 1.0],
        ),
    ];

    if !debug {
        return out;
    }

    let px = sw - 310.0;
    let mut py = 28.0;
    let lh = 13.0;

    let spd = game.player.movement.speed * game.player.movement.velocity_frac;
    out.push(ts(
        px,
        py,
        format!(
            "[DEBUG]  spd:{:.2} tiles/s  enemies:{}",
            spd,
            game.enemies.len()
        ),
        12.0,
        [0.9, 0.9, 0.2, 1.0],
    ));
    py += lh + 2.0;

    let player_pos = (game.player.movement.x, game.player.movement.y);
    let room_info = match game.rooms.find_room_at(player_pos.0, player_pos.1) {
        Some(room_idx) => format!("Room: {}", room_idx),
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

    for (i, e) in game.enemies.iter().enumerate() {
        if e.kind == EnemyKind::TargetDummy {
            out.push(ts(
                px,
                py,
                format!("T{i} [TARGET] hp:{}", e.hp),
                11.0,
                [1.0, 0.85, 0.25, 1.0],
            ));
            py += lh + 3.0;
            continue;
        }

        let state_label = match e.brain.awareness.state {
            AiState::Combat => "COMBAT",
            AiState::Alert => "ALERT ",
            AiState::Idle => "idle  ",
        };
        let sees = if e.brain.awareness.state == AiState::Combat {
            "Y"
        } else {
            "n"
        };
        let col = if e.brain.awareness.in_combat() {
            [1.0, 0.35, 0.35, 1.0]
        } else if e.brain.awareness.is_alert() {
            [1.0, 0.65, 0.2, 1.0]
        } else {
            [0.55, 0.8, 0.55, 1.0]
        };
        out.push(ts(
            px,
            py,
            format!(
                "E{i} [{state_label}] susp:{:.2} hp:{} vis:{}",
                e.brain.awareness.suspicion, e.hp, sees
            ),
            11.0,
            col,
        ));
        py += lh;

        let pos = (e.movement.x, e.movement.y);
        let anchor = e.brain.spawn_anchor();
        out.push(ts(
            px + 8.0,
            py,
            format!(
                "pos:({:.0},{:.0}) anc:({:.0},{:.0})",
                pos.0, pos.1, anchor.0, anchor.1
            ),
            10.0,
            [0.6, 0.6, 0.6, 1.0],
        ));
        py += lh;

        out.push(ts(
            px + 8.0,
            py,
            format!("phase:{}", e.brain.phase_name()),
            10.0,
            [0.5, 0.75, 1.0, 1.0],
        ));
        py += lh;

        if let Some(lk) = e.brain.awareness.last_known_pos() {
            out.push(ts(
                px + 8.0,
                py,
                format!("last_known:({:.0},{:.0})", lk.0, lk.1),
                10.0,
                [0.85, 0.5, 0.85, 1.0],
            ));
            py += lh;
        }

        if let Some(mv) = e.brain.last_move_target {
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
