/// Player lobby command sent from client → server (2 bytes on the wire).
///
/// Wire layout: [kind: u8] [team: u8]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LobbyCommand {
    /// Which command should be applied.
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

/// Current lobby state sent from server → client (4 bytes on the wire).
///
/// Wire layout: [game_started: u8] [your_team: u8] [team1_count: u8] [team2_count: u8]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LobbyState {
    /// `true` after the match has started; no new clients can join.
    pub game_started: bool,
    /// Team selection for this client (`0` = none, `1` = team 1, `2` = team 2).
    pub your_team: u8,
    /// Number of players currently assigned to team 1.
    pub team1_count: u8,
    /// Number of players currently assigned to team 2.
    pub team2_count: u8,
}
