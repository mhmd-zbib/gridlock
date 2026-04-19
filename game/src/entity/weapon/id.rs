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

    /// Returns the position of this weapon in the shared catalog.
    /// Identical on client and server (both load the same sorted JSON files).
    pub fn catalog_index(self) -> usize {
        self.0
    }
}
