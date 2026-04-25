/// Render view types — flat, pre-assembled data for each render function.
///
/// Render functions never import `Game` or any game logic type.
/// All view assembly (including server-authoritative overrides) happens in
/// `app/render.rs` — the single translation boundary between game and render.
use engine::asset::AssetHandle;
use game::world::level::LevelBounds;
use game::world::wall::Wall;
use net::PlayerState;

// ---------------------------------------------------------------------------
// Fog (stencil mask)
// ---------------------------------------------------------------------------

pub struct TeammateConeFog {
    pub pos: (f32, f32),
    pub sight_direction: f32,
    pub sight_half_angle: f32,
    pub sight_range: f32,
    pub sight_circle_radius: f32,
}

pub struct FogView<'a> {
    pub player_pos: (f32, f32),
    /// Already resolved: server-authoritative rotation if connected, else local.
    pub sight_direction: f32,
    pub sight_half_angle: f32,
    pub sight_range: f32,
    pub sight_circle_radius: f32,
    /// Sight cones of living teammates — unioned into the fog mask.
    pub teammate_cones: Vec<TeammateConeFog>,
    pub walls: &'a [Wall],
}

// ---------------------------------------------------------------------------
// Geometry overlays
// ---------------------------------------------------------------------------

pub struct SightConeView {
    pub pos: (f32, f32),
    pub direction: f32,
    pub half_angle: f32,
    pub range: f32,
    pub circle_radius: f32,
}

pub struct AimConeView {
    /// Already resolved: server-authoritative aim direction if connected.
    pub direction: f32,
    /// Already resolved: server-authoritative half-angle if connected.
    pub half_angle: f32,
    pub render_range: f32,
}

pub struct ImpactView {
    pub pos: (f32, f32),
    /// Pre-computed normalised lifetime [0,1] — 1 = fresh, 0 = expired.
    pub alpha: f32,
}

pub struct EnemyConeView {
    pub pos: (f32, f32),
    /// Already converted to screen pixels.
    pub circle_radius_px: f32,
    pub circle_color: [f32; 4],
    /// Pre-computed from awareness state + visibility.
    pub cone_color: [f32; 4],
    pub sight_direction: f32,
    pub sight_half_angle: f32,
    pub sight_range: f32,
}

pub struct DebugRoomView {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub color: [f32; 4],
}

pub struct DebugRoomsView {
    pub rooms: Vec<DebugRoomView>,
    pub gaps: Vec<(f32, f32)>,
}

pub struct PlayerCircleView {
    pub pos: (f32, f32),
    pub color: [f32; 4],
}

pub struct GeometryView<'a> {
    pub walls: &'a [Wall],
    pub player_sight: SightConeView,
    pub aim_cone: AimConeView,
    pub impacts: Vec<ImpactView>,
    pub enemy_cones: Vec<EnemyConeView>,
    pub teammate_cones: Vec<TeammateConeFog>,
    /// All players rendered as circles: local player first, then remote players.
    pub player_circles: Vec<PlayerCircleView>,
    /// `Some` only when debug mode is active.
    pub debug_rooms: Option<DebugRoomsView>,
    /// True when the local player is dead/spectating — suppresses the aim cone.
    pub is_spectating: bool,
}

// ---------------------------------------------------------------------------
// Entity quads
// ---------------------------------------------------------------------------

pub struct EnemyDebugView {
    pub spawn_anchor: (f32, f32),
    pub last_known_pos: Option<(f32, f32)>,
    pub last_move_target: Option<(f32, f32)>,
    pub gap_waypoints: Vec<(f32, f32)>,
}

pub struct EnemyBodyView {
    pub pos: (f32, f32),
    /// Base color (RGB). Alpha carries the temporally-smoothed vis_alpha for
    /// visible enemies, or the fixed dim alpha for teammate-cone-only enemies.
    pub color: [f32; 4],
    /// True when the enemy is visible through the local player's own cone.
    /// Determines which render path: cone_vis (soft per-pixel) vs. masked (dim binary).
    pub is_visible_to_player: bool,
    /// `Some` only when debug mode is active and enemy kind supports it.
    pub debug: Option<EnemyDebugView>,
}

pub struct EntitiesView<'a> {
    pub enemies: Vec<EnemyBodyView>,
    pub bullet_positions: Vec<(f32, f32)>,
    pub remote_players: &'a [PlayerState],
    /// Player position in screen pixels — origin for cone visibility math.
    pub vis_viewer_pos: (f32, f32),
    /// Normalised forward vector of the player's sight cone (screen space).
    pub vis_view_dir: (f32, f32),
    /// cos(half_angle − feather): inner angular boundary, fully visible inside.
    pub vis_cos_inner: f32,
    /// cos(half_angle): outer angular boundary, fully dark at/beyond.
    pub vis_cos_outer: f32,
    /// Distance in screen pixels at which cone brightness starts to fall off.
    pub vis_range_inner_px: f32,
    /// Maximum cone range in screen pixels — zero brightness beyond this.
    pub vis_range_outer_px: f32,
    /// Near-halo radius in screen pixels — always visible within this disk.
    pub vis_circle_radius_px: f32,
}

// ---------------------------------------------------------------------------
// World quads
// ---------------------------------------------------------------------------

pub struct FloorView {
    pub pos: (f32, f32),
    pub half_size: (f32, f32),
    pub color: [f32; 4],
    pub texture_handle: Option<AssetHandle>,
}

pub struct PropView {
    pub pos: (f32, f32),
    pub half_size: (f32, f32),
    /// Fallback solid color used when `texture_handle` is `None`.
    pub color: [f32; 4],
    /// Some → render as a textured sprite; None → render as a color quad.
    pub texture_handle: Option<AssetHandle>,
}

pub struct WorldView<'a> {
    pub level_bounds: Option<LevelBounds>,
    pub walls: &'a [Wall],
    pub floors: Vec<FloorView>,
    pub props: Vec<PropView>,
}

// ---------------------------------------------------------------------------
// HUD text
// ---------------------------------------------------------------------------

pub struct HudPlayerView {
    pub weapon_name: &'static str,
    pub weapon_class: &'static str,
    pub ammo: u8,
    pub mag_size: u32,
    pub reloading: bool,
    pub attachments_line: String,
    pub speed: f32,
    pub pos: (f32, f32),
    pub room_idx: Option<usize>,
    pub enemy_count: usize,
}

pub struct HudEnemyRow {
    pub idx: usize,
    pub is_dummy: bool,
    pub hp: u32,
    pub state_label: &'static str,
    pub suspicion: f32,
    pub in_combat: bool,
    pub color: [f32; 4],
    pub pos: (f32, f32),
    pub anchor: (f32, f32),
    pub phase: &'static str,
    pub last_known_pos: Option<(f32, f32)>,
    pub last_move_target: Option<(f32, f32)>,
}

pub struct HudView<'a> {
    pub player: HudPlayerView,
    pub enemies: Vec<HudEnemyRow>,
    pub net: Option<&'a crate::net::NetClient>,
    pub health: u8,
    pub match_state: Option<net::MatchState>,
    /// Kill notification text shown for 3 seconds after death.
    pub kill_notification: Option<String>,
}
