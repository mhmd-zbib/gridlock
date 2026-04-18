/// Player lobby command sent from client → server (2 bytes on the wire).
///
/// Wire layout: [kind: u8] [team: u8]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LobbyCommand {
    pub kind: LobbyCommandKind,
    /// Team index in `[1, 2]` for [`LobbyCommandKind::SelectTeam`], ignored for
    /// [`LobbyCommandKind::StartGame`].
    pub team: u8,
}

impl LobbyCommand {
    pub fn select_team(team: u8) -> Self {
        Self {
            kind: LobbyCommandKind::SelectTeam,
            team: if team == 2 { 2 } else { 1 },
        }
    }

    pub fn start_game() -> Self {
        Self {
            kind: LobbyCommandKind::StartGame,
            team: 0,
        }
    }
}

/// Discriminant for [`LobbyCommand`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LobbyCommandKind {
    SelectTeam = 0,
    StartGame = 1,
}

impl LobbyCommandKind {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::SelectTeam),
            1 => Some(Self::StartGame),
            _ => None,
        }
    }
}

// ── LobbyPlayer ───────────────────────────────────────────────────────────────

/// One player entry in the lobby roster (17 bytes on the wire).
///
/// Wire layout: [name: [u8; 16]] [team: u8]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LobbyPlayer {
    /// Display name as UTF-8, zero-padded to 16 bytes.
    pub name: [u8; 16],
    /// Team (`0` = none, `1` = team 1, `2` = team 2).
    pub team: u8,
}

impl LobbyPlayer {
    pub fn name_str(&self) -> &str {
        let end = self.name.iter().position(|&b| b == 0).unwrap_or(16);
        std::str::from_utf8(&self.name[..end]).unwrap_or("?")
    }
}

// ── LobbyState ────────────────────────────────────────────────────────────────

/// Current lobby state sent from server → client.
///
/// Wire layout:
///   [game_started: u8] [your_team: u8] [team1_count: u8] [team2_count: u8]
///   [is_creator: u8]
///   [player_count: u8] [players: player_count × 17 B]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LobbyState {
    /// `true` once the creator has pressed Start Game.
    pub game_started: bool,
    /// Team selection for this client (`0` = none, `1` = team 1, `2` = team 2).
    pub your_team: u8,
    /// Number of non-spectator players assigned to team 1.
    pub team1_count: u8,
    /// Number of non-spectator players assigned to team 2.
    pub team2_count: u8,
    /// Whether this client is the room creator (can press Start Game).
    pub is_creator: bool,
    /// All players currently in the session for roster display.
    pub players: Vec<LobbyPlayer>,
}
