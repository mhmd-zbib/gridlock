pub mod client;
pub mod handshake;
pub mod lobby;
pub mod server;

// ── MoveSpeed ─────────────────────────────────────────────────────────────────

/// Movement speed tier shared by [`client::InputFlags`] and
/// [`server::MovementState`].
///
/// ```text
/// 0 = SlowWalk  (crouched / sneak)
/// 1 = Walk      (default pace)
/// 2 = Run       (sprint)
/// 3 = reserved
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum MoveSpeed {
    SlowWalk = 0,
    #[default]
    Walk = 1,
    Run = 2,
}

impl MoveSpeed {
    /// Decodes the lower 2 bits of `v`; unknown value 3 falls back to `Walk`.
    pub fn from_u8(v: u8) -> Self {
        match v & 0b11 {
            0 => Self::SlowWalk,
            1 => Self::Walk,
            2 => Self::Run,
            _ => Self::Walk,
        }
    }
}
