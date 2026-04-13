use crate::core::entity::player::PlayerLoadoutConfig;
use crate::core::entity::weapon::attachment::{
    ATTACHMENT_CATEGORY_COUNT, AttachmentCategory, AttachmentId, AttachmentLoadout, attachment_name,
};
use crate::core::entity::weapon::{
    WeaponId, all_weapon_ids, weapon_attachment_categories, weapon_attachment_ids_for_category,
    weapon_supports_attachment_category,
};
use crate::input::InputState;
use crate::render::quad::QuadInstance;

const ROW_COUNT: usize = ATTACHMENT_CATEGORY_COUNT + 1;

pub struct LoadoutMenu {
    weapon_ids: Vec<WeaponId>,
    selected_weapon_index: usize,
    selected_row: usize,
    attachments: AttachmentLoadout,
    attachment_indices: [usize; ATTACHMENT_CATEGORY_COUNT], // 0 = none, 1.. = category option index + 1
    prev_up: bool,
    prev_down: bool,
    prev_left: bool,
    prev_right: bool,
}

impl LoadoutMenu {
    pub fn new(initial: &PlayerLoadoutConfig) -> Self {
        let weapon_ids = all_weapon_ids();
        assert!(!weapon_ids.is_empty(), "weapon catalog is empty");

        let selected_weapon_index = weapon_ids
            .iter()
            .position(|id| *id == initial.weapon)
            .unwrap_or(0);

        let mut out = Self {
            weapon_ids,
            selected_weapon_index,
            selected_row: 0,
            attachments: initial.attachments.clone(),
            attachment_indices: [0; ATTACHMENT_CATEGORY_COUNT],
            prev_up: false,
            prev_down: false,
            prev_left: false,
            prev_right: false,
        };

        for category in AttachmentCategory::all() {
            let options = weapon_attachment_ids_for_category(out.selected_weapon_id(), *category);
            let slot = category.index();
            if let Some(current) = out.attachments.get(*category) {
                if let Some(idx) = options.iter().position(|candidate| candidate == current) {
                    out.attachment_indices[slot] = idx + 1;
                } else {
                    out.attachments.unequip(*category);
                }
            }
        }
        out.normalize_for_selected_weapon();

        println!("[loadout] LOADOUT BUILDER");
        println!("[loadout]   ↑/↓ select row   ←/→ change option");
        println!("[loadout]   Esc/Enter: back to menu");

        out
    }

    pub fn update(&mut self, input: &InputState) {
        if input.up && !self.prev_up {
            self.selected_row = (self.selected_row + ROW_COUNT - 1) % ROW_COUNT;
        }
        if input.down && !self.prev_down {
            self.selected_row = (self.selected_row + 1) % ROW_COUNT;
        }
        if input.left && !self.prev_left {
            self.step_selected_option(-1);
        }
        if input.right && !self.prev_right {
            self.step_selected_option(1);
        }

        self.prev_up = input.up;
        self.prev_down = input.down;
        self.prev_left = input.left;
        self.prev_right = input.right;
    }

    pub fn selected_config(&self) -> PlayerLoadoutConfig {
        PlayerLoadoutConfig {
            weapon: self.weapon_ids[self.selected_weapon_index],
            attachments: self.attachments.clone(),
        }
    }

    pub fn selected_row(&self) -> usize {
        self.selected_row
    }

    pub fn selected_weapon_name(&self) -> &'static str {
        self.weapon_ids[self.selected_weapon_index].stats().name
    }

    pub fn selected_weapon_class_label(&self) -> &'static str {
        self.weapon_ids[self.selected_weapon_index]
            .stats()
            .class
            .label()
    }

    pub fn selected_attachment_name(&self, category: AttachmentCategory) -> String {
        if !self.selected_weapon_supports(category) {
            return "(not supported)".to_string();
        }
        self.attachments
            .get(category)
            .and_then(attachment_name)
            .map(str::to_string)
            .unwrap_or_else(|| "(none)".to_string())
    }

    pub fn selected_weapon_supports(&self, category: AttachmentCategory) -> bool {
        weapon_supports_attachment_category(self.selected_weapon_id(), category)
    }

    pub fn instances(&self, sw: f32, sh: f32) -> Vec<QuadInstance> {
        let mut out = Vec::new();

        out.push(QuadInstance {
            center: [sw * 0.5, sh * 0.5],
            half_size: [sw * 0.36, sh * 0.32],
            color: [0.12, 0.12, 0.12, 1.0],
        });

        let bw = sw * 0.64;
        let bh = (sh * 0.55 / ROW_COUNT as f32).min(48.0);
        let gap = 8.0;
        let start_y = sh * 0.24;
        let cx = sw * 0.5;
        for row in 0..ROW_COUNT {
            let y = start_y + row as f32 * (bh + gap);
            out.push(QuadInstance {
                center: [cx, y + bh * 0.5],
                half_size: [bw * 0.5, bh * 0.5],
                color: if row == self.selected_row {
                    [0.88, 0.88, 0.88, 1.0]
                } else {
                    [0.24, 0.24, 0.24, 1.0]
                },
            });
        }

        let arrow_y = start_y + self.selected_row as f32 * (bh + gap) + bh * 0.5;
        out.push(QuadInstance {
            center: [sw * 0.5 - bw * 0.5 - 12.0, arrow_y],
            half_size: [5.0, 5.0],
            color: [1.0, 0.9, 0.15, 1.0],
        });

        out
    }

    fn step_selected_option(&mut self, direction: i8) {
        if self.selected_row == 0 {
            let len = self.weapon_ids.len();
            if direction > 0 {
                self.selected_weapon_index = (self.selected_weapon_index + 1) % len;
            } else {
                self.selected_weapon_index = (self.selected_weapon_index + len - 1) % len;
            }
            self.normalize_for_selected_weapon();
            return;
        }

        let category = AttachmentCategory::all()[self.selected_row - 1];
        if !self.selected_weapon_supports(category) {
            return;
        }
        let options = weapon_attachment_ids_for_category(self.selected_weapon_id(), category);
        let slot = category.index();
        let total = options.len() + 1; // + none option
        if total == 0 {
            return;
        }

        if direction > 0 {
            self.attachment_indices[slot] = (self.attachment_indices[slot] + 1) % total;
        } else {
            self.attachment_indices[slot] = (self.attachment_indices[slot] + total - 1) % total;
        }

        let idx = self.attachment_indices[slot];
        if idx == 0 {
            self.attachments.unequip(category);
            return;
        }

        let selected: AttachmentId = options[idx - 1].clone();
        self.attachments
            .equip_in_slot(category, selected)
            .expect("category list should only contain matching-category attachments");
    }

    fn selected_weapon_id(&self) -> WeaponId {
        self.weapon_ids[self.selected_weapon_index]
    }

    fn normalize_for_selected_weapon(&mut self) {
        let weapon = self.selected_weapon_id();
        let supported_categories = weapon_attachment_categories(weapon);
        for category in AttachmentCategory::all() {
            let slot = category.index();
            if !supported_categories.contains(category) {
                self.attachment_indices[slot] = 0;
                self.attachments.unequip(*category);
                continue;
            }

            let options = weapon_attachment_ids_for_category(weapon, *category);
            if let Some(current) = self.attachments.get(*category) {
                if let Some(idx) = options.iter().position(|candidate| candidate == current) {
                    self.attachment_indices[slot] = idx + 1;
                    continue;
                }
                self.attachments.unequip(*category);
            }
            self.attachment_indices[slot] = 0;
        }
    }
}
