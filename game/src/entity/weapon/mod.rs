pub mod attachment;
mod class;
mod id;
mod state;
mod stats;
mod weapons;
use crate::entity::weapon::attachment::{AttachmentCategory, AttachmentId};
pub use class::WeaponClass;
pub use id::WeaponId;
pub use state::WeaponState;
pub use stats::WeaponStats;

pub fn all_weapon_ids() -> Vec<WeaponId> {
    weapons::all_ids()
}

pub fn weapon_attachment_categories(id: WeaponId) -> &'static [AttachmentCategory] {
    weapons::attachment_categories(id)
}

pub fn weapon_supports_attachment_category(id: WeaponId, category: AttachmentCategory) -> bool {
    weapons::supports_attachment_category(id, category)
}

pub fn weapon_attachment_ids_for_category(
    id: WeaponId,
    category: AttachmentCategory,
) -> &'static [AttachmentId] {
    weapons::attachment_ids_for_category(id, category)
}

pub fn weapon_supports_attachment(id: WeaponId, attachment: &AttachmentId) -> bool {
    weapons::supports_attachment(id, attachment)
}
