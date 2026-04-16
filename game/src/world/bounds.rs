use crate::world::level::LevelBounds;

/// Clamp an actor's position so it stays within the level bounds.
///
/// If the bounds are narrower than the actor's diameter the actor is placed at
/// the centre of the bounds axis so it cannot escape to infinity.
pub fn clamp_actor_to_level_bounds(
    x: &mut f32,
    y: &mut f32,
    half: f32,
    bounds: Option<LevelBounds>,
) {
    let Some(bounds) = bounds else {
        return;
    };
    let min_x = bounds.x + half;
    let max_x = bounds.x + bounds.w - half;
    let min_y = bounds.y + half;
    let max_y = bounds.y + bounds.h - half;

    if min_x <= max_x {
        *x = x.clamp(min_x, max_x);
    } else {
        *x = bounds.x + bounds.w * 0.5;
    }
    if min_y <= max_y {
        *y = y.clamp(min_y, max_y);
    } else {
        *y = bounds.y + bounds.h * 0.5;
    }
}
