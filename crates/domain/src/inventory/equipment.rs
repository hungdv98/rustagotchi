//! What a pet is wearing.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::item::EquipmentSlot;
use crate::identifiers::ItemId;

/// One item per slot.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Equipment {
    worn: BTreeMap<EquipmentSlot, ItemId>,
}

impl Equipment {
    #[must_use]
    pub fn new() -> Self {
        Equipment::default()
    }

    #[must_use]
    pub fn in_slot(&self, slot: EquipmentSlot) -> Option<&ItemId> {
        self.worn.get(&slot)
    }

    /// Puts `item` in `slot`, returning whatever it displaced.
    ///
    /// Returning the displaced item rather than refusing means the caller has to
    /// deal with it — normally by putting it back in the inventory — instead of
    /// it silently vanishing.
    #[must_use = "the displaced item must be returned to the inventory"]
    pub fn equip(&mut self, slot: EquipmentSlot, item: ItemId) -> Option<ItemId> {
        self.worn.insert(slot, item)
    }

    #[must_use = "the removed item must be returned to the inventory"]
    pub fn unequip(&mut self, slot: EquipmentSlot) -> Option<ItemId> {
        self.worn.remove(&slot)
    }

    #[must_use]
    pub fn is_equipped(&self, item: &ItemId) -> bool {
        self.worn.values().any(|worn| worn == item)
    }

    /// Ordered by slot, always.
    pub fn iter(&self) -> impl Iterator<Item = (EquipmentSlot, &ItemId)> {
        self.worn.iter().map(|(slot, item)| (*slot, item))
    }

    #[must_use]
    pub fn count(&self) -> usize {
        self.worn.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(name: &str) -> ItemId {
        ItemId::new(name).expect("valid item id")
    }

    #[test]
    fn equipping_fills_the_named_slot() {
        let mut equipment = Equipment::new();
        assert!(
            equipment
                .equip(EquipmentSlot::Head, item("straw_hat"))
                .is_none()
        );
        assert_eq!(
            equipment.in_slot(EquipmentSlot::Head),
            Some(&item("straw_hat"))
        );
        assert_eq!(equipment.in_slot(EquipmentSlot::Body), None);
    }

    #[test]
    fn equipping_over_an_occupied_slot_hands_back_the_old_item() {
        let mut equipment = Equipment::new();
        let _ = equipment.equip(EquipmentSlot::Head, item("straw_hat"));
        let displaced = equipment.equip(EquipmentSlot::Head, item("steel_helm"));
        assert_eq!(
            displaced,
            Some(item("straw_hat")),
            "the old item must not vanish"
        );
        assert_eq!(equipment.count(), 1);
    }

    #[test]
    fn unequipping_returns_the_item_and_frees_the_slot() {
        let mut equipment = Equipment::new();
        let _ = equipment.equip(EquipmentSlot::Body, item("cloak"));
        assert_eq!(equipment.unequip(EquipmentSlot::Body), Some(item("cloak")));
        assert_eq!(equipment.unequip(EquipmentSlot::Body), None);
        assert_eq!(equipment.count(), 0);
    }

    #[test]
    fn slots_are_independent() {
        let mut equipment = Equipment::new();
        for slot in EquipmentSlot::ALL {
            let _ = equipment.equip(slot, item("charm"));
        }
        assert_eq!(equipment.count(), EquipmentSlot::ALL.len());
    }

    #[test]
    fn worn_items_are_detectable() {
        let mut equipment = Equipment::new();
        let _ = equipment.equip(EquipmentSlot::Accessory, item("bell"));
        assert!(equipment.is_equipped(&item("bell")));
        assert!(!equipment.is_equipped(&item("cloak")));
    }
}
