use serde::{Deserialize, Serialize};

pub const ATTACHMENT_CATEGORY_COUNT: usize = 6;
pub const ALL_ATTACHMENT_CATEGORIES: [AttachmentCategory; ATTACHMENT_CATEGORY_COUNT] = [
    AttachmentCategory::Optic,
    AttachmentCategory::Barrel,
    AttachmentCategory::Grip,
    AttachmentCategory::Magazine,
    AttachmentCategory::Stock,
    AttachmentCategory::Underbarrel,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttachmentCategory {
    Optic,
    Barrel,
    Grip,
    Magazine,
    Stock,
    Underbarrel,
}

impl AttachmentCategory {
    pub fn all() -> &'static [AttachmentCategory; ATTACHMENT_CATEGORY_COUNT] {
        &ALL_ATTACHMENT_CATEGORIES
    }

    pub fn label(self) -> &'static str {
        match self {
            AttachmentCategory::Optic => "Optic",
            AttachmentCategory::Barrel => "Barrel",
            AttachmentCategory::Grip => "Grip",
            AttachmentCategory::Magazine => "Magazine",
            AttachmentCategory::Stock => "Stock",
            AttachmentCategory::Underbarrel => "Underbarrel",
        }
    }

    pub fn index(self) -> usize {
        match self {
            AttachmentCategory::Optic => 0,
            AttachmentCategory::Barrel => 1,
            AttachmentCategory::Grip => 2,
            AttachmentCategory::Magazine => 3,
            AttachmentCategory::Stock => 4,
            AttachmentCategory::Underbarrel => 5,
        }
    }
}
