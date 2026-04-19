/// Input system: build ClientPacket from InputState, mouse-world conversion.
pub mod input;
/// Network system: drain server snapshots, apply authoritative state.
pub mod network;
/// Client prediction + reconciliation for local player movement.
pub mod prediction;
