use ecs::Component;

use crate::ai::brain::EnemyBrain;
use crate::entity::bullet::BulletOwner;
use crate::entity::enemy::EnemyKind;
use crate::entity::player::locomotion::LocomotionState;
use crate::entity::player::loadout::WeaponLoadout;
use crate::world::aim_cone::AimCone;

// ---------------------------------------------------------------------------
// Transform components
// ---------------------------------------------------------------------------

#[derive(Default, Clone, Copy)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

impl From<(f32, f32)> for Position {
    fn from((x, y): (f32, f32)) -> Self {
        Self { x, y }
    }
}

impl From<Position> for (f32, f32) {
    fn from(p: Position) -> Self {
        (p.x, p.y)
    }
}

impl Component for Position {}

// Constant-direction velocity (used by bullets).
#[derive(Default, Clone, Copy)]
pub struct Velocity {
    pub x: f32,
    pub y: f32,
}

impl Component for Velocity {}

// Maximum movement speed in tiles/second.
#[derive(Default, Clone, Copy)]
pub struct Speed(pub f32);

impl Component for Speed {}

// Normalized 0..1 fraction of max speed (used by sound and animation).
#[derive(Default, Clone, Copy)]
pub struct VelocityFrac(pub f32);

impl Component for VelocityFrac {}

// Collision half-extent radius in tile-space.
#[derive(Default, Clone, Copy)]
pub struct CollisionRadius(pub f32);

impl Component for CollisionRadius {}

// ---------------------------------------------------------------------------
// Combat components
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
pub struct Health {
    pub current: u32,
    pub max: u32,
}

impl Health {
    pub fn new(hp: u32) -> Self {
        Self { current: hp, max: hp }
    }

    pub fn is_dead(self) -> bool {
        self.current == 0
    }
}

impl Component for Health {}

#[derive(Default, Clone, Copy)]
pub struct Damage(pub u32);

impl Component for Damage {}

// ---------------------------------------------------------------------------
// Lifetime component
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
pub struct Lifetime {
    pub remaining: f32,
}

impl Lifetime {
    pub fn new(secs: f32) -> Self {
        Self { remaining: secs }
    }

    pub fn is_expired(self) -> bool {
        self.remaining <= 0.0
    }
}

impl Component for Lifetime {}

// ---------------------------------------------------------------------------
// Visibility component
// ---------------------------------------------------------------------------

/// Set by VisibilitySystem for every entity that can be observed.
#[derive(Default, Clone, Copy)]
pub struct VisibilityState {
    /// Scalar in [0, 1]: 0 = not visible, 1 = fully visible.
    pub score: f32,
}

impl VisibilityState {
    pub fn is_visible(self) -> bool {
        self.score > 0.0
    }
}

impl Component for VisibilityState {}

// ---------------------------------------------------------------------------
// Bullet-specific data
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
pub struct BulletData {
    pub origin: (f32, f32),
    pub owner: BulletOwner,
}

impl Component for BulletData {}

// ---------------------------------------------------------------------------
// Player controller component
// ---------------------------------------------------------------------------

/// Player-specific controller state: weapon management and locomotion.
/// Lives alongside Position, Speed, VelocityFrac, Sight, AimCone, SoundField.
pub struct PlayerController {
    pub loadout: WeaponLoadout,
    pub locomotion: LocomotionState,
    pub aim_cone: AimCone,
}

impl PlayerController {
    pub fn new() -> Self {
        Self {
            loadout: WeaponLoadout::new(),
            locomotion: LocomotionState::new(),
            aim_cone: AimCone::new(),
        }
    }
}

impl Component for PlayerController {}

// ---------------------------------------------------------------------------
// AI agent component
// ---------------------------------------------------------------------------

pub struct AiAgent {
    pub brain: EnemyBrain,
    pub kind: EnemyKind,
}

impl AiAgent {
    pub fn new(x: f32, y: f32, direction: f32, kind: EnemyKind) -> Self {
        Self {
            brain: EnemyBrain::new((x, y), direction),
            kind,
        }
    }
}

impl Component for AiAgent {}

// ---------------------------------------------------------------------------
// Tag components (zero-sized markers)
// ---------------------------------------------------------------------------

pub struct PlayerTag;
impl Component for PlayerTag {}

pub struct BotTag;
impl Component for BotTag {}

pub struct BulletTag;
impl Component for BulletTag {}

pub struct ImpactTag;
impl Component for ImpactTag {}
