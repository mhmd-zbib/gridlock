mod catalog;
mod category;
mod effects;
mod loadout;

pub use catalog::{
    AttachmentDef, all_attachment_defs, all_attachment_ids, attachment_effects,
    attachment_id_by_id, attachment_id_by_name, attachment_ids_for_category, attachment_name,
};
pub use category::{ALL_ATTACHMENT_CATEGORIES, ATTACHMENT_CATEGORY_COUNT, AttachmentCategory};
pub use effects::{
    AccuracyEffects, AttachmentEffects, HandlingEffects, MobilityEffects, PowerEffects,
    RecoilEffects, StealthEffects, UtilityEffects,
};
pub use loadout::{AttachmentEquipError, AttachmentId, AttachmentLoadout};

#[cfg(test)]
mod tests {
    use super::{
        ALL_ATTACHMENT_CATEGORIES, AccuracyEffects, AttachmentCategory, AttachmentEffects,
        AttachmentEquipError, AttachmentId, AttachmentLoadout, all_attachment_defs,
        all_attachment_ids, attachment_effects, attachment_id_by_id, attachment_id_by_name,
        attachment_ids_for_category, attachment_name,
    };

    #[test]
    fn attachment_categories_are_fixed_and_complete() {
        assert_eq!(
            ALL_ATTACHMENT_CATEGORIES,
            [
                AttachmentCategory::Optic,
                AttachmentCategory::Barrel,
                AttachmentCategory::Grip,
                AttachmentCategory::Magazine,
                AttachmentCategory::Stock,
                AttachmentCategory::Underbarrel,
            ]
        );
    }

    #[test]
    fn attachment_catalog_loads_from_json() {
        let defs = all_attachment_defs();
        let ids = all_attachment_ids();
        assert!(!defs.is_empty());
        assert_eq!(defs.len(), ids.len());
        assert!(defs.iter().any(|def| {
            def.id == "red_dot"
                && def.category == AttachmentCategory::Optic
                && def.name == "Red Dot"
        }));
    }

    #[test]
    fn attachment_lookup_by_name_and_id_is_case_flexible() {
        let from_name = attachment_id_by_name("Red Dot").expect("Red Dot should exist");
        let from_id = attachment_id_by_name("RED_DOT").expect("red_dot should exist");
        let by_id = attachment_id_by_id("ReD_DoT").expect("red_dot should exist");
        assert_eq!(from_name.id(), "red_dot");
        assert_eq!(from_name.category(), AttachmentCategory::Optic);
        assert_eq!(from_id.id(), "red_dot");
        assert_eq!(by_id.id(), "red_dot");
        assert_eq!(attachment_name(&from_id), Some("Red Dot"));
    }

    #[test]
    fn each_fixed_category_has_at_least_one_catalog_entry() {
        for category in AttachmentCategory::all() {
            assert!(
                !attachment_ids_for_category(*category).is_empty(),
                "missing attachment for category {category:?}"
            );
        }
    }

    #[test]
    fn effects_can_be_read_for_attachment() {
        let laser = AttachmentId::new("laser_pointer", AttachmentCategory::Underbarrel);
        let fx = attachment_effects(&laser).expect("laser_pointer should have effects");
        assert!(fx.accuracy.hip_fire_spread_mult < 1.0);
        assert!(fx.utility.ping_visibility_mult > 1.0);
    }

    #[test]
    fn effect_combine_multiplies_values() {
        let a = AttachmentEffects {
            accuracy: AccuracyEffects {
                hip_fire_spread_mult: 0.9,
                ..Default::default()
            },
            ..Default::default()
        };
        let b = AttachmentEffects {
            accuracy: AccuracyEffects {
                hip_fire_spread_mult: 0.8,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = a.combine(b);
        assert!((out.accuracy.hip_fire_spread_mult - 0.72).abs() < 0.0001);
    }

    #[test]
    fn empty_slot_is_none_and_equip_sets_one_active_attachment() {
        let mut loadout = AttachmentLoadout::default();
        assert!(loadout.is_empty(AttachmentCategory::Optic));

        let optic = AttachmentId::new("red_dot", AttachmentCategory::Optic);
        assert_eq!(loadout.equip(optic), None);
        assert_eq!(
            loadout.get(AttachmentCategory::Optic).map(|a| a.id()),
            Some("red_dot")
        );
    }

    #[test]
    fn replacing_attachment_overrides_previous_one_in_same_slot() {
        let mut loadout = AttachmentLoadout::default();

        let first = AttachmentId::new("red_dot", AttachmentCategory::Optic);
        let second = AttachmentId::new("holo_sight", AttachmentCategory::Optic);

        assert_eq!(loadout.equip(first), None);
        let replaced = loadout.equip(second);

        assert_eq!(
            replaced.map(|a| a.id().to_string()),
            Some("red_dot".to_string())
        );
        assert_eq!(
            loadout.get(AttachmentCategory::Optic).map(|a| a.id()),
            Some("holo_sight")
        );
    }

    #[test]
    fn cross_category_equip_is_rejected() {
        let mut loadout = AttachmentLoadout::default();
        let attachment = AttachmentId::new("red_dot", AttachmentCategory::Optic);

        let err = loadout
            .equip_in_slot(AttachmentCategory::Barrel, attachment)
            .expect_err("category mismatch should fail");

        assert_eq!(
            err,
            AttachmentEquipError::CategoryMismatch {
                slot: AttachmentCategory::Barrel,
                attachment: AttachmentCategory::Optic,
            }
        );
    }
}
