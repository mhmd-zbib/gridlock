pub mod loadout;
pub mod locomotion;

use crate::entity::weapon::attachment::{AttachmentCategory, AttachmentLoadout};
use crate::entity::weapon::{
    WeaponId, all_weapon_ids, weapon_supports_attachment, weapon_supports_attachment_category,
};

#[derive(Clone)]
pub struct PlayerLoadoutConfig {
    pub weapon: WeaponId,
    pub attachments: AttachmentLoadout,
}

impl Default for PlayerLoadoutConfig {
    fn default() -> Self {
        let all = all_weapon_ids();
        let Some(first_weapon) = all.first().copied() else {
            panic!("weapon catalog is empty");
        };
        Self {
            weapon: first_weapon,
            attachments: AttachmentLoadout::default(),
        }
    }
}

impl PlayerLoadoutConfig {
    pub fn sanitize(&mut self) {
        for category in AttachmentCategory::all() {
            if weapon_supports_attachment_category(self.weapon, *category) {
                if let Some(attachment) = self.attachments.get(*category) {
                    if !weapon_supports_attachment(self.weapon, attachment) {
                        self.attachments.unequip(*category);
                    }
                }
            } else {
                self.attachments.unequip(*category);
            }
        }
    }
}
