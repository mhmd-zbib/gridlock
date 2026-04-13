use super::{WeaponStats, weapons};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WeaponId(pub(crate) usize);

impl WeaponId {
    pub fn stats(self) -> WeaponStats {
        weapons::stats(self)
    }

    pub fn from_name(name: &str) -> Option<Self> {
        weapons::id_by_name(name)
    }
}
