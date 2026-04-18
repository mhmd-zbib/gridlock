#[derive(Clone, Copy, Debug)]
pub struct ScreenTransform {
    pub center: (f32, f32),
    pub viewport_px: (f32, f32),
    pub pixels_per_unit: f32,
}

impl ScreenTransform {
    pub fn new(center: (f32, f32), viewport_px: (f32, f32), pixels_per_unit: f32) -> Self {
        Self {
            center,
            viewport_px,
            pixels_per_unit,
        }
    }

    pub fn world_to_screen(&self, world: (f32, f32)) -> (f32, f32) {
        (
            (world.0 - self.center.0) * self.pixels_per_unit + self.viewport_px.0 * 0.5,
            (world.1 - self.center.1) * self.pixels_per_unit + self.viewport_px.1 * 0.5,
        )
    }

    pub fn screen_to_world(&self, screen_px: (f32, f32)) -> (f32, f32) {
        (
            (screen_px.0 - self.viewport_px.0 * 0.5) / self.pixels_per_unit + self.center.0,
            (screen_px.1 - self.viewport_px.1 * 0.5) / self.pixels_per_unit + self.center.1,
        )
    }

    pub fn world_points_to_screen<I>(&self, points: I) -> Vec<[f32; 2]>
    where
        I: IntoIterator<Item = [f32; 2]>,
    {
        points
            .into_iter()
            .map(|p| {
                let s = self.world_to_screen((p[0], p[1]));
                [s.0, s.1]
            })
            .collect()
    }
}
