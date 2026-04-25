use ecs::{Entity, Event};

use crate::entity::bullet::BulletOwner;

// ---------------------------------------------------------------------------
// Spawn events
// ---------------------------------------------------------------------------

pub struct SpawnBulletEvent {
    pub x: f32,
    pub y: f32,
    /// Normalised direction vector.
    pub dir_x: f32,
    pub dir_y: f32,
    pub speed: f32,
    pub damage: u32,
    pub owner: BulletOwner,
}

impl Event for SpawnBulletEvent {}

pub struct SpawnEnemyEvent {
    pub x: f32,
    pub y: f32,
}

impl Event for SpawnEnemyEvent {}

// ---------------------------------------------------------------------------
// Destruction event
// ---------------------------------------------------------------------------

pub struct DestroyEvent {
    pub entity: Entity,
}

impl Event for DestroyEvent {}

// ---------------------------------------------------------------------------
// Combat events
// ---------------------------------------------------------------------------

pub struct DamageEvent {
    pub target: Entity,
    pub amount: u32,
    pub source: Entity,
}

impl Event for DamageEvent {}

// ---------------------------------------------------------------------------
// Bullet trace (for server snapshot)
// ---------------------------------------------------------------------------

/// Records the muzzle origin and impact position of a player bullet so the
/// server can broadcast a BulletEvent in the same tick the shot was fired.
#[derive(Clone, Copy)]
pub struct BulletTraceEvent {
    pub x: f32,
    pub y: f32,
    pub origin_x: f32,
    pub origin_y: f32,
}

impl Event for BulletTraceEvent {}

// ---------------------------------------------------------------------------
// Sound events
// ---------------------------------------------------------------------------

pub struct SoundEvent {
    pub source: Entity,
    pub radius: f32,
    pub position: (f32, f32),
}

impl Event for SoundEvent {}

// ---------------------------------------------------------------------------
// Visibility events
// ---------------------------------------------------------------------------

pub struct VisibilityChangedEvent {
    pub observer: Entity,
    pub target: Entity,
    /// Visibility score in [0, 1].
    pub score: f32,
}

impl Event for VisibilityChangedEvent {}
