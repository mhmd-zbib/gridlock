use crate::core::entity::bullet::BulletOwner;

/// A request to create a new entity, pushed during the update loop
/// and flushed by `Game` at the end of each frame.
///
/// Every system (player, wave spawner, level loader, …) pushes here.
/// Nobody creates entities directly — they just describe what they want.
pub enum SpawnRequest {
    Bullet {
        x: f32,
        y: f32,
        /// Normalised direction vector.
        dir_x: f32,
        dir_y: f32,
        speed: f32,
        damage: u32,
        owner: BulletOwner,
    },
    Enemy {
        x: f32,
        y: f32,
    },
    // Extend with Player { .. }, Object { kind, x, y }, etc. as needed.
}

/// Collects spawn requests for one frame. Cleared after each flush.
#[derive(Default)]
pub struct SpawnQueue {
    pending: Vec<SpawnRequest>,
}

impl SpawnQueue {
    pub fn push(&mut self, req: SpawnRequest) {
        self.pending.push(req);
    }

    /// Drain all pending requests. Call once per frame after all updates.
    pub fn drain(&mut self) -> std::vec::Drain<'_, SpawnRequest> {
        self.pending.drain(..)
    }
}
