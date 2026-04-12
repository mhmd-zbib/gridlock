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
        match self {
            WeaponId::Ak47 => weapons::ak47::stats(),
            WeaponId::Mp5 => weapons::mp5::stats(),
            WeaponId::Sniper => weapons::sniper::stats(),
            WeaponId::M4a1 => weapons::m4a1::stats(),
            WeaponId::Uzi => weapons::uzi::stats(),
            WeaponId::Dmr => weapons::dmr::stats(),
        }
    }
}
