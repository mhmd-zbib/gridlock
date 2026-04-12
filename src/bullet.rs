/// Bullets don't respond to input — they move in a fixed direction
/// at a fixed speed, so a velocity vector is enough. No MovementInput needed.
pub struct Bullet {
    pub x: f32,
    pub y: f32,
    vx: f32,
    vy: f32,
    pub alive: bool,
}

const BULLET_SPEED: f32 = 600.0;

impl Bullet {
    /// `dir_x / dir_y` must be a normalised direction vector.
    pub fn new(x: f32, y: f32, dir_x: f32, dir_y: f32) -> Self {
        Self {
            x,
            y,
            vx: dir_x * BULLET_SPEED,
            vy: dir_y * BULLET_SPEED,
            alive: true,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.x += self.vx * dt;
        self.y += self.vy * dt;
    }
}
