mod defaults;
mod loader;

use crate::entity::weapon::attachment::{
    ATTACHMENT_CATEGORY_COUNT, AttachmentCategory, AttachmentId,
};
use crate::entity::weapon::{WeaponId, WeaponStats};
use std::sync::OnceLock;

use loader::load_catalog;

#[derive(Clone)]
pub(super) struct WeaponEntry {
    pub(super) id: &'static str,
    pub(super) stats: WeaponStats,
    pub(super) attachments: [Vec<AttachmentId>; ATTACHMENT_CATEGORY_COUNT],
    pub(super) attachment_categories: Vec<AttachmentCategory>,
}

static CATALOG: OnceLock<Vec<WeaponEntry>> = OnceLock::new();

pub fn stats(id: WeaponId) -> WeaponStats {
    catalog()
        .get(id.0)
        .unwrap_or_else(|| panic!("invalid weapon id index: {}", id.0))
        .stats
}

pub fn all_ids() -> Vec<WeaponId> {
    (0..catalog().len()).map(WeaponId).collect()
}

pub fn attachment_categories(id: WeaponId) -> &'static [AttachmentCategory] {
    catalog()
        .get(id.0)
        .unwrap_or_else(|| panic!("invalid weapon id index: {}", id.0))
        .attachment_categories
        .as_slice()
}

pub fn attachment_ids_for_category(
    id: WeaponId,
    category: AttachmentCategory,
) -> &'static [AttachmentId] {
    catalog()
        .get(id.0)
        .unwrap_or_else(|| panic!("invalid weapon id index: {}", id.0))
        .attachments[category.index()]
    .as_slice()
}

pub fn supports_attachment_category(id: WeaponId, category: AttachmentCategory) -> bool {
    !attachment_ids_for_category(id, category).is_empty()
}

pub fn supports_attachment(id: WeaponId, attachment: &AttachmentId) -> bool {
    attachment_ids_for_category(id, attachment.category())
        .iter()
        .any(|allowed| allowed == attachment)
}

pub fn id_by_name(name: &str) -> Option<WeaponId> {
    catalog()
        .iter()
        .position(|entry| entry.stats.name == name || entry.id.eq_ignore_ascii_case(name))
        .map(WeaponId)
}

fn catalog() -> &'static [WeaponEntry] {
    CATALOG.get_or_init(load_catalog).as_slice()
}

#[cfg(test)]
mod tests {
    use super::{attachment_ids_for_category, supports_attachment, supports_attachment_category};
    use crate::entity::weapon::WeaponId;
    use crate::entity::weapon::attachment::{AttachmentCategory, AttachmentId};

    fn weapon(name: &str) -> WeaponId {
        WeaponId::from_name(name).unwrap_or_else(|| panic!("missing weapon in catalog: {name}"))
    }

    #[test]
    fn rifles_support_underbarrel_but_snipers_do_not() {
        let ak = weapon("AK-47");
        let sniper = weapon("Sniper");
        assert!(supports_attachment_category(
            ak,
            AttachmentCategory::Underbarrel
        ));
        assert!(!supports_attachment_category(
            sniper,
            AttachmentCategory::Underbarrel
        ));
    }

    #[test]
    fn unsupported_category_is_rejected_for_smg() {
        let mp5 = weapon("MP5");
        assert!(!supports_attachment_category(mp5, AttachmentCategory::Grip));
        assert!(supports_attachment_category(
            mp5,
            AttachmentCategory::Magazine
        ));
    }

    #[test]
    fn weapon_attachment_ids_are_category_specific() {
        let ak = weapon("AK-47");
        let optics = attachment_ids_for_category(ak, AttachmentCategory::Optic);
        assert!(optics.iter().any(|id| id.id() == "red_dot"));
        assert!(optics.iter().any(|id| id.id() == "holo_sight"));
    }

    #[test]
    fn weapon_rejects_attachment_not_listed_in_json() {
        let sniper = weapon("Sniper");
        let laser = AttachmentId::new("laser_pointer", AttachmentCategory::Underbarrel);
        assert!(!supports_attachment(sniper, &laser));
    }
}
