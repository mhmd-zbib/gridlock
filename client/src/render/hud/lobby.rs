use crate::net::NetClient;
use game::render::text::TextSection;
use net::LobbyState;

use super::shared::{net_status_line, ts};

pub fn lobby_texts(
    sw: f32,
    sh: f32,
    net: Option<&NetClient>,
    lobby: Option<&LobbyState>,
) -> Vec<TextSection> {
    let game_started = lobby.map(|s| s.game_started).unwrap_or(false);
    let your_team = lobby.map(|s| s.your_team).unwrap_or(0);
    let is_creator = lobby.map(|s| s.is_creator).unwrap_or(false);
    let team1_count = lobby.map(|s| s.team1_count).unwrap_or(0);
    let team2_count = lobby.map(|s| s.team2_count).unwrap_or(0);

    let bw = sw * 0.34;
    let bh = sh * 0.10;
    // Buttons are placed higher to leave room for player names below them.
    let team1_x = sw * 0.5 - bw - sw * 0.03;
    let team2_x = sw * 0.5 + sw * 0.03;
    let team_btn_y = sh * 0.30;
    let team_label_y = team_btn_y + bh * 0.28;
    let start_btn_y = sh * 0.70;

    let title = if game_started {
        "GAME IN PROGRESS — PICK A TEAM TO SPECTATE"
    } else {
        "WAITING FOR PLAYERS"
    };
    let subtitle = if game_started {
        "You will join on the next round"
    } else if is_creator {
        "Choose a team, then press Start Game"
    } else {
        "Choose a team and wait for the creator to start"
    };

    let selected_team_line = match your_team {
        1 => "Selected: TEAM 1",
        2 => "Selected: TEAM 2",
        _ => "No team selected yet",
    };

    let net_line = net_status_line(net);

    let mut out = vec![
        ts(
            sw * 0.5 - 280.0,
            sh * 0.16,
            title,
            30.0,
            [1.0, 1.0, 1.0, 1.0],
        ),
        ts(
            sw * 0.5 - 240.0,
            sh * 0.23,
            subtitle,
            16.0,
            [0.72, 0.72, 0.72, 1.0],
        ),
        // Team button labels
        ts(
            team1_x + bw * 0.5 - 72.0,
            team_label_y,
            format!("TEAM 1 ({team1_count})"),
            26.0,
            [0.0, 0.0, 0.0, 1.0],
        ),
        ts(
            team2_x + bw * 0.5 - 72.0,
            team_label_y,
            format!("TEAM 2 ({team2_count})"),
            26.0,
            [0.0, 0.0, 0.0, 1.0],
        ),
    ];

    // Start button label — only shown to the creator before the game begins.
    if is_creator && !game_started {
        out.push(ts(
            sw * 0.5 - 86.0,
            start_btn_y + bh * 0.28,
            "START GAME",
            28.0,
            [0.0, 0.0, 0.0, 1.0],
        ));
    } else if !game_started {
        out.push(ts(
            sw * 0.5 - 160.0,
            start_btn_y + bh * 0.28,
            "Waiting for creator to start…",
            18.0,
            [0.65, 0.65, 0.65, 1.0],
        ));
    }

    // Player roster — names under each team column.
    if let Some(lobby) = lobby {
        let name_start_y = team_btn_y + bh + 6.0;
        let row_h = 18.0;

        let team1_names: Vec<&str> = lobby
            .players
            .iter()
            .filter(|p| p.team == 1)
            .map(|p| p.name_str())
            .collect();
        let team2_names: Vec<&str> = lobby
            .players
            .iter()
            .filter(|p| p.team == 2)
            .map(|p| p.name_str())
            .collect();

        for (i, name) in team1_names.iter().enumerate().take(8) {
            out.push(ts(
                team1_x + 8.0,
                name_start_y + i as f32 * row_h,
                *name,
                14.0,
                [0.55, 0.95, 0.55, 1.0],
            ));
        }
        for (i, name) in team2_names.iter().enumerate().take(8) {
            out.push(ts(
                team2_x + 8.0,
                name_start_y + i as f32 * row_h,
                *name,
                14.0,
                [0.55, 0.70, 1.0, 1.0],
            ));
        }
    }

    // Status lines at the bottom of the panel.
    out.push(ts(
        sw * 0.5 - 100.0,
        sh * 0.83,
        selected_team_line,
        18.0,
        [0.85, 0.85, 0.85, 1.0],
    ));
    out.push(ts(
        sw * 0.5 - 110.0,
        sh * 0.87,
        net_line,
        15.0,
        [0.62, 0.62, 0.62, 1.0],
    ));
    out.push(ts(
        sw * 0.5 - 155.0,
        sh * 0.91,
        "Esc: back to main menu",
        14.0,
        [0.50, 0.50, 0.50, 1.0],
    ));

    out
}
