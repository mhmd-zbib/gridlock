use super::{WeaponStats, weapons};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeaponId {
    Ak47,
    Mp5,
    Sniper,
    M4a1,
    Uzi,
    Dmr,
}

impl WeaponId {
    pub fn stats(self) -> WeaponStats {
        weapons::stats(self)
    }
}
