use crate::net::NetClient;
use engine::render::text::TextSection;
use net::LobbyState;

use super::shared::{net_status_line, ts};

pub fn lobby_texts(
    sw: f32,
    sh: f32,
    net: Option<&NetClient>,
    lobby: Option<&LobbyState>,
) -> Vec<TextSection> {
    let team1_count = lobby.map(|s| s.team1_count).unwrap_or(0);
    let team2_count = lobby.map(|s| s.team2_count).unwrap_or(0);
    let selected_team = match lobby.map(|s| s.your_team).unwrap_or(0) {
        1 => "Selected team: TEAM 1".to_string(),
        2 => "Selected team: TEAM 2".to_string(),
        _ => "Selected team: (none yet)".to_string(),
    };
    let waiting = if lobby.map(|s| s.game_started).unwrap_or(false) {
        "Match is starting..."
    } else {
        "Waiting room"
    };
    let net_line = net_status_line(net);

    let bw = sw * 0.34;
    let bh = sh * 0.10;
    let team1_x = sw * 0.5 - bw - sw * 0.03;
    let team2_x = sw * 0.5 + sw * 0.03;
    let team_y = sh * 0.44 + bh * 0.28;
    let start_y = sh * 0.62 + bh * 0.28;

    vec![
        ts(
            sw * 0.5 - 200.0,
            sh * 0.24,
            "WAITING FOR PLAYERS",
            44.0,
            [1.0, 1.0, 1.0, 1.0],
        ),
        ts(
            sw * 0.5 - 150.0,
            sh * 0.30,
            "Choose a team, then press Start Game",
            18.0,
            [0.72, 0.72, 0.72, 1.0],
        ),
        ts(
            team1_x + bw * 0.5 - 65.0,
            team_y,
            format!("TEAM 1 ({team1_count})"),
            28.0,
            [0.0, 0.0, 0.0, 1.0],
        ),
        ts(
            team2_x + bw * 0.5 - 65.0,
            team_y,
            format!("TEAM 2 ({team2_count})"),
            28.0,
            [0.0, 0.0, 0.0, 1.0],
        ),
        ts(
            sw * 0.5 - 86.0,
            start_y,
            "START GAME",
            30.0,
            [0.0, 0.0, 0.0, 1.0],
        ),
        ts(
            sw * 0.5 - 95.0,
            sh * 0.78,
            selected_team,
            20.0,
            [0.85, 0.85, 0.85, 1.0],
        ),
        ts(
            sw * 0.5 - 52.0,
            sh * 0.82,
            waiting,
            18.0,
            [0.72, 0.72, 0.72, 1.0],
        ),
        ts(
            sw * 0.5 - 110.0,
            sh * 0.86,
            net_line,
            16.0,
            [0.62, 0.62, 0.62, 1.0],
        ),
        ts(
            sw * 0.5 - 155.0,
            sh * 0.90,
            "Esc: back to main menu",
            15.0,
            [0.55, 0.55, 0.55, 1.0],
        ),
    ]
}
