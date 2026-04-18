use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use game::entity::weapon::WeaponState;
use game::world::aim_cone::AimCone;
use net::ClientPacket;
use net::proto::server::MovementState;

// ---------------------------------------------------------------------------
// Session
// ---------------------------------------------------------------------------

/// Per-connected-player state maintained by the server.
pub struct Session {
    pub player_id: u16,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub health: u16,
    /// Rotation encoded as a u16 angle (see `net::codec`).
    pub rotation: u16,
    pub weapon: WeaponState,
    pub aim_cone: AimCone,
    pub movement_state: MovementState,
    /// Team selection: `0` = none, `1` = team 1, `2` = team 2.
    pub team: u8,
    /// `true` for players who joined after the match started or who have not
    /// yet been spawned into the round (mid-game joiners).
    pub is_spectator: bool,
    /// Latest input packet received from this client (updated every tick).
    pub latest_input: Option<ClientPacket>,
}

// ---------------------------------------------------------------------------
// ServerState
// ---------------------------------------------------------------------------

pub struct ServerState {
    pub sessions: HashMap<SocketAddr, Session>,
    pub next_player_id: u16,
    pub game_started: bool,
    /// World-space spawn point applied to newly connected players.
    pub spawn: (f32, f32),
    /// Team-specific spawns; `None` means fall back to `spawn`.
    pub team1_spawn: Option<(f32, f32)>,
    pub team2_spawn: Option<(f32, f32)>,
    /// Rounds won by each team.
    pub team1_rounds: u8,
    pub team2_rounds: u8,
    /// Seconds remaining in the respawn countdown; 0.0 = normal play.
    pub respawn_timer: f32,
    /// Address of the player who created the room; only they can start the game.
    pub creator_addr: Option<SocketAddr>,
}

/// Seconds between a round ending and the next round starting.
pub const RESPAWN_DELAY: f32 = 5.0;

impl ServerState {
    pub fn new() -> Self {
        use game::world::units::px_to_tiles;
        Self {
            sessions: HashMap::new(),
            next_player_id: 1,
            game_started: false,
            spawn: (px_to_tiles(400.0), px_to_tiles(300.0)),
            team1_spawn: None,
            team2_spawn: None,
            team1_rounds: 0,
            team2_rounds: 0,
            respawn_timer: 0.0,
            creator_addr: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Shared
// ---------------------------------------------------------------------------

/// Thread-safe reference to the mutable server state shared between the main
/// tick loop and the async receive task.
pub type Shared = Arc<Mutex<ServerState>>;
