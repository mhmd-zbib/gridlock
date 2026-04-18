/// Entity quads: player, enemies, remote players, bullets.
pub mod entities;
/// Fog of war: stencil-buffer triangle fans for the vision cone mask.
pub mod fog;
/// Geometry overlays: sight cones, aim cone, bullet tracers, debug shapes.
pub mod geometry;
/// HUD text: weapon info, ammo, debug panel, and per-screen text labels.
pub mod hud;
/// Ray-cast geometry builders for sight circles, cones, and aim cones.
pub mod sight_geometry;
/// Render view-model structs used as a translation boundary from game state.
pub mod views;
/// World quads: floor, walls, and props.
pub mod world;
