pub const TILE_SIZE_PX: f32 = 64.0;

#[inline]
pub const fn px_to_tiles(px: f32) -> f32 {
    px / TILE_SIZE_PX
}

#[inline]
pub const fn tiles_to_px(tiles: f32) -> f32 {
    tiles * TILE_SIZE_PX
}
