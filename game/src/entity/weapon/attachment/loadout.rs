use serde::{Deserialize, Serialize};

use super::category::{ATTACHMENT_CATEGORY_COUNT, AttachmentCategory};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AttachmentId {
    id: String,
    category: AttachmentCategory,
}

impl AttachmentId {
    pub fn new(id: impl Into<String>, category: AttachmentCategory) -> Self {
        Self {
            id: id.into(),
            category,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn category(&self) -> AttachmentCategory {
        self.category
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AttachmentEquipError {
    CategoryMismatch {
        slot: AttachmentCategory,
        attachment: AttachmentCategory,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentLoadout {
    slots: [Option<AttachmentId>; ATTACHMENT_CATEGORY_COUNT],
}

impl Default for AttachmentLoadout {
    fn default() -> Self {
        Self {
            slots: std::array::from_fn(|_| None),
        }
    }
}

impl AttachmentLoadout {
    pub fn is_empty(&self, category: AttachmentCategory) -> bool {
        self.get(category).is_none()
    }

    pub fn get(&self, category: AttachmentCategory) -> Option<&AttachmentId> {
        self.slots[category.index()].as_ref()
    }

    pub fn unequip(&mut self, category: AttachmentCategory) -> Option<AttachmentId> {
        self.slots[category.index()].take()
    }

    pub fn equip(&mut self, attachment: AttachmentId) -> Option<AttachmentId> {
        self.replace(attachment.category(), attachment)
    }

    pub fn equip_in_slot(
        &mut self,
        slot: AttachmentCategory,
        attachment: AttachmentId,
    ) -> Result<Option<AttachmentId>, AttachmentEquipError> {
        if slot != attachment.category() {
            return Err(AttachmentEquipError::CategoryMismatch {
                slot,
                attachment: attachment.category(),
            });
        }
        Ok(self.replace(slot, attachment))
    }

    fn replace(
        &mut self,
        slot: AttachmentCategory,
        attachment: AttachmentId,
    ) -> Option<AttachmentId> {
        let idx = slot.index();
        let mut replaced = Some(attachment);
        std::mem::swap(&mut self.slots[idx], &mut replaced);
        replaced
    }
}
