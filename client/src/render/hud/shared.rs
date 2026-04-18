use crate::net::{ConnState, NetClient};
use game::render::text::TextSection;

pub(super) fn ts(
    x: f32,
    y: f32,
    text: impl Into<String>,
    size: f32,
    color: [f32; 4],
) -> TextSection {
    TextSection {
        x,
        y,
        text: text.into(),
        size,
        color,
    }
}

pub(super) fn net_status_line(net: Option<&NetClient>) -> String {
    match net.map(|n| n.state()) {
        None => "net: off".into(),
        Some(ConnState::Connecting) => "net: connecting…".into(),
        Some(ConnState::Connected { player_id }) => format!("net: player #{player_id}"),
        Some(ConnState::Rejected(r)) => format!("net: rejected ({r})"),
        Some(ConnState::Disconnected) => "net: disconnected".into(),
    }
}
