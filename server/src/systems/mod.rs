/// Combat system: weapon ticking, ray-cast hit resolution, BulletEvent emission.
pub mod combat;
/// Movement system: apply client input, wall collision, level-bounds clamping.
pub mod movement;
/// Round system: detect team elimination, manage respawn countdown.
pub mod rounds;
/// Snapshot system: build per-client ServerPacket from authoritative state.
pub mod snapshot;
