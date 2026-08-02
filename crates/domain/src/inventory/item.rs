//! Item classification.
//!
//! Only what the *rules* need. An item's name, sprite, price, restoration
//! amounts and flavour text are content (ADR 0005); what lives here is the
//! handful of distinctions gameplay actually branches on.

use core::fmt;

use serde::{Deserialize, Serialize};

/// What an item is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ItemCategory {
    Food,
    Medicine,
    Toy,
    Equipment,
    Material,
}

impl ItemCategory {
    pub const ALL: [ItemCategory; 5] = [
        ItemCategory::Food,
        ItemCategory::Medicine,
        ItemCategory::Toy,
        ItemCategory::Equipment,
        ItemCategory::Material,
    ];

    /// True if using one consumes it.
    #[must_use]
    pub const fn is_consumable(self) -> bool {
        matches!(self, ItemCategory::Food | ItemCategory::Medicine)
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            ItemCategory::Food => "food",
            ItemCategory::Medicine => "medicine",
            ItemCategory::Toy => "toy",
            ItemCategory::Equipment => "equipment",
            ItemCategory::Material => "material",
        }
    }
}

impl fmt::Display for ItemCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Where a piece of equipment goes. One item per slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EquipmentSlot {
    Head,
    Body,
    Accessory,
}

impl EquipmentSlot {
    pub const ALL: [EquipmentSlot; 3] = [
        EquipmentSlot::Head,
        EquipmentSlot::Body,
        EquipmentSlot::Accessory,
    ];

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            EquipmentSlot::Head => "head",
            EquipmentSlot::Body => "body",
            EquipmentSlot::Accessory => "accessory",
        }
    }
}

impl fmt::Display for EquipmentSlot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_food_and_medicine_are_consumed_on_use() {
        assert!(ItemCategory::Food.is_consumable());
        assert!(ItemCategory::Medicine.is_consumable());
        assert!(!ItemCategory::Toy.is_consumable());
        assert!(!ItemCategory::Equipment.is_consumable());
        assert!(!ItemCategory::Material.is_consumable());
    }

    #[test]
    fn every_variant_is_listed_and_named() {
        assert_eq!(ItemCategory::ALL.len(), 5);
        assert_eq!(EquipmentSlot::ALL.len(), 3);
        for category in ItemCategory::ALL {
            assert!(!category.name().is_empty());
        }
    }
}
