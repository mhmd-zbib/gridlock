/// Full game-state snapshot sent from server → client every tick.
///
/// Wire layout (big-endian):
///   [tick: u32] [timestamp: u32]
///   [self_state: 18 B]
///   [player_count: u8] [players: player_count × 14 B]
///   [bullet_count: u8] [bullets: bullet_count × 20 B]
///   [sound_count:  u8] [sounds:  sound_count  × 10 B]
///   [teammate_count: u8] [teammates: teammate_count × 22 B]
///   [match_state: 4 B]
///
/// Maximum wire size with reasonable counts (64 players, 32 bullets, 16 sounds, 4 teammates):
///   8 + 18 + 1 + 64×14 + 1 + 32×20 + 1 + 16×10 + 1 + 4×22 + 4 = 1,825 B
/// Keep player/bullet/sound counts small to stay under the 1 400 B MTU ceiling.
#[derive(Debug, Clone)]
pub struct ServerPacket {
    /// Server authoritative tick counter; monotonically increasing.
    pub tick: u32,

    /// The `ClientPacket::timestamp` of the most-recently processed input from
    /// this client, echoed back for RTT measurement.
    pub timestamp: u32,

    /// State specific to the receiving client's own player.
    pub me: SelfState,

    /// All other visible players (does **not** include the receiving client).
    /// Teammates are always included regardless of line-of-sight.
    pub players: Vec<PlayerState>,

    /// Projectile traces resolved this tick (for hit-scan / visual feedback).
    pub bullets: Vec<BulletEvent>,

    /// Spatial sound events the client should play.
    pub sounds: Vec<SoundEvent>,

    /// Sight cones of living teammates, for shared-vision fog-of-war rendering.
    pub teammate_views: Vec<TeammateView>,

    /// Current match metadata.
    pub match_state: MatchState,
}

// ── TeammateView ──────────────────────────────────────────────────────────────

/// Sight cone of one living teammate (22 bytes on the wire).
///
/// Wire layout: [x: f32] [y: f32] [rotation: u16]
///              [sight_range: f32] [sight_half_angle: f32] [sight_circle_radius: f32]
#[derive(Debug, Clone, Copy)]
pub struct TeammateView {
    pub x: f32,
    pub y: f32,
    /// Aim direction compressed to `[0, 65535]`.
    pub rotation: u16,
    pub sight_range: f32,
    /// In radians.
    pub sight_half_angle: f32,
    pub sight_circle_radius: f32,
}

// ── SelfState ─────────────────────────────────────────────────────────────────

/// The local player's authoritative state (18 bytes on the wire).
///
/// Wire layout: [health: u8] [ammo: u8] [reload_progress: u8] [movement_state: u8]
///              [x: f32] [y: f32] [rotation: u16] [aim_cone_half_angle: f32]
#[derive(Debug, Clone, Copy, Default)]
pub struct SelfState {
    /// Current health; `0` means dead.
    pub health: u8,

    /// Rounds remaining in the active magazine.
    pub ammo: u8,

    /// Reload progress: `0` = not reloading, `255` = fully complete.
    /// Derived from `WeaponState::reload_left / reload_time * 255`.
    pub reload_progress: u8,

    /// Packed locomotion + action flags; see [`MovementState`].
    pub movement_state: MovementState,

    /// Server-authoritative position X in tiles.
    pub x: f32,

    /// Server-authoritative position Y in tiles.
    pub y: f32,

    /// Aim direction compressed to `[0, 65535]` (same encoding as
    /// `ClientPacket::rotation`).
    pub rotation: u16,

    /// Current authoritative aim-cone half-angle in radians.
    pub aim_cone_half_angle: f32,
}

// ── PlayerState ───────────────────────────────────────────────────────────────

/// One remote player's state (15 bytes on the wire).
///
/// Wire layout: [id: u16] [x: f32] [y: f32] [rotation: u16] [movement_state: u8] [weapon: u8] [team: u8]
#[derive(Debug, Clone, Copy)]
pub struct PlayerState {
    /// Server-assigned player identifier (`0` is reserved / invalid).
    pub id: u16,

    /// World-space position X in tiles.
    pub x: f32,

    /// World-space position Y in tiles.
    pub y: f32,

    /// Aim direction compressed to `[0, 65535]` (same encoding as
    /// `ClientPacket::rotation`).
    pub rotation: u16,

    /// Packed locomotion flags; see [`MovementState`].
    pub movement_state: MovementState,

    /// Index into the server's weapon catalogue (`WeaponId` cast to `u8`).
    pub weapon: u8,

    /// Team assignment (`0` = none, `1` = team 1, `2` = team 2).
    pub team: u8,
}

// ── BulletEvent ───────────────────────────────────────────────────────────────

/// A hitscan trace resolved this tick (21 bytes on the wire).
///
/// Wire layout:
///   [shooter_id: u16] [from_x: f32] [from_y: f32]
///   [to_x: f32] [to_y: f32] [hit_player_id: u16] [shooter_team: u8]
#[derive(Debug, Clone, Copy)]
pub struct BulletEvent {
    /// `PlayerState::id` of the shooter (`0` = environment / unknown).
    pub shooter_id: u16,

    /// Muzzle position X in tiles.
    pub from_x: f32,

    /// Muzzle position Y in tiles.
    pub from_y: f32,

    /// Impact (or max-range) position X in tiles.
    pub to_x: f32,

    /// Impact (or max-range) position Y in tiles.
    pub to_y: f32,

    /// Victim's `PlayerState::id`; `0` means the shot missed all players.
    pub hit_player_id: u16,

    /// Shooter's team (`0` = no team / unknown).
    pub shooter_team: u8,
}

// ── SoundEvent ────────────────────────────────────────────────────────────────

/// A spatial audio event the client should play (10 bytes on the wire).
///
/// Wire layout: [kind: u8] [x: f32] [y: f32] [intensity: u8]
#[derive(Debug, Clone, Copy)]
pub struct SoundEvent {
    /// Sound category; see [`SoundKind`].
    pub kind: SoundKind,

    /// World-space position X in tiles.
    pub x: f32,

    /// World-space position Y in tiles.
    pub y: f32,

    /// Volume/falloff hint in `[0, 255]`; maps to `shot_sound_radius` etc.
    pub intensity: u8,
}

// ── MatchState ────────────────────────────────────────────────────────────────

/// Match-level metadata (4 bytes on the wire).
///
/// Wire layout: [timer: u16] [score_team1: u8] [score_team2: u8]
#[derive(Debug, Clone, Copy, Default)]
pub struct MatchState {
    /// Remaining match time in seconds; `0` means the round has ended.
    pub timer: u16,

    /// Current score for team 1.
    pub score_team1: u8,

    /// Current score for team 2.
    pub score_team2: u8,
}

// ── MovementState ─────────────────────────────────────────────────────────────

/// Bitfield shared by [`SelfState::movement_state`] and
/// [`PlayerState::movement_state`] (1 byte on the wire).
///
/// ```text
/// bits 0-1: MOVE_SPEED  (0 = SlowWalk · 1 = Walk · 2 = Run · 3 = reserved)
/// bit 2   : PEEKING
/// bit 3   : RELOADING
/// bits 4-7: reserved
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MovementState(pub u8);

impl MovementState {
    const MOVE_SPEED_MASK: u8 = 0b0000_0011;
    const PEEKING: u8 = 1 << 2;
    const RELOADING: u8 = 1 << 3;

    /// Returns the movement speed tier.
    pub fn move_speed(self) -> super::MoveSpeed {
        super::MoveSpeed::from_u8(self.0 & Self::MOVE_SPEED_MASK)
    }
    pub fn is_peeking(self) -> bool {
        self.0 & Self::PEEKING != 0
    }
    pub fn is_reloading(self) -> bool {
        self.0 & Self::RELOADING != 0
    }

    /// Encodes the movement speed tier into bits 0-1.
    pub fn set_move_speed(&mut self, speed: super::MoveSpeed) {
        self.0 = (self.0 & !Self::MOVE_SPEED_MASK) | (speed as u8 & Self::MOVE_SPEED_MASK);
    }
    pub fn set_peeking(&mut self, v: bool) {
        self.set_bit(Self::PEEKING, v);
    }
    pub fn set_reloading(&mut self, v: bool) {
        self.set_bit(Self::RELOADING, v);
    }

    fn set_bit(&mut self, bit: u8, v: bool) {
        if v {
            self.0 |= bit;
        } else {
            self.0 &= !bit;
        }
    }
}

// ── SoundKind ─────────────────────────────────────────────────────────────────

/// Discriminant for [`SoundEvent::kind`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SoundKind {
    Gunshot = 0,
    Reload = 1,
    Footstep = 2,
    Hit = 3,
    Death = 4,
}

impl SoundKind {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Gunshot),
            1 => Some(Self::Reload),
            2 => Some(Self::Footstep),
            3 => Some(Self::Hit),
            4 => Some(Self::Death),
            _ => None,
        }
    }
}
