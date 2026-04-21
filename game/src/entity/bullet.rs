#[derive(Clone, Copy, PartialEq)]
pub enum BulletOwner {
    Player,
    Enemy,
}

/// Bullets don't respond to input — they move in a fixed direction
/// at a fixed speed, so a velocity vector is enough. No MovementInput needed.
pub struct Bullet {
    pub x: f32,
    pub y: f32,
    /// World-space position where this bullet was spawned (muzzle position).
    pub origin_x: f32,
    pub origin_y: f32,
    vx: f32,
    vy: f32,
    pub damage: u32,
    pub alive: bool,
    pub owner: BulletOwner,
}

impl Bullet {
    /// `dir_x / dir_y` must be a normalised direction vector.
    pub fn new(
        x: f32,
        y: f32,
        dir_x: f32,
        dir_y: f32,
        speed: f32,
        damage: u32,
        owner: BulletOwner,
    ) -> Self {
        Self {
            x,
            y,
            origin_x: x,
            origin_y: y,
            vx: dir_x * speed,
            vy: dir_y * speed,
            damage,
            alive: true,
            owner,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.x += self.vx * dt;
        self.y += self.vy * dt;
    }

    /// Normalised direction the bullet is travelling.
    pub fn dir(&self) -> (f32, f32) {
        let len = (self.vx * self.vx + self.vy * self.vy).sqrt();
        if len > 1e-9 { (self.vx / len, self.vy / len) } else { (0.0, 0.0) }
    }
}
