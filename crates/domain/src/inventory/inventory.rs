//! The player's inventory.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::error::DomainError;
use crate::identifiers::ItemId;

/// What the player is carrying.
///
/// Capacity counts *distinct item kinds*, not total units — one slot per kind,
/// as in the classic games. `BTreeMap` keeps iteration ordered so that a UI list
/// and a save digest are both stable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Inventory {
    slots: BTreeMap<ItemId, u32>,
    capacity: usize,
}

impl Inventory {
    pub const DEFAULT_CAPACITY: usize = 64;

    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Inventory {
            slots: BTreeMap::new(),
            capacity,
        }
    }

    #[must_use]
    pub fn count(&self, item: &ItemId) -> u32 {
        self.slots.get(item).copied().unwrap_or(0)
    }

    #[must_use]
    pub fn has_at_least(&self, item: &ItemId, quantity: u32) -> bool {
        self.count(item) >= quantity
    }

    #[must_use]
    pub fn distinct_kinds(&self) -> usize {
        self.slots.len()
    }

    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    #[must_use]
    pub fn is_full(&self) -> bool {
        self.slots.len() >= self.capacity
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// Ordered by item id, always.
    pub fn iter(&self) -> impl Iterator<Item = (&ItemId, u32)> {
        self.slots.iter().map(|(item, quantity)| (item, *quantity))
    }

    /// Adds `quantity` of `item`.
    ///
    /// # Errors
    ///
    /// [`DomainError::AtCapacity`] if the item is not already held and there is
    /// no free slot. Topping up a kind already carried always succeeds, so a
    /// full inventory never blocks a reward the player can obviously hold.
    pub fn add(&mut self, item: ItemId, quantity: u32) -> Result<(), DomainError> {
        if quantity == 0 {
            return Ok(());
        }
        // Read the current amount out before touching the map, so the capacity
        // check below is not fighting a live mutable borrow.
        match self.slots.get(&item).copied() {
            Some(existing) => {
                self.slots.insert(item, existing.saturating_add(quantity));
                Ok(())
            }
            None if self.is_full() => Err(DomainError::AtCapacity {
                container: "inventory",
                capacity: self.capacity,
            }),
            None => {
                self.slots.insert(item, quantity);
                Ok(())
            }
        }
    }

    /// Removes `quantity` of `item`.
    ///
    /// All or nothing: a partial removal would leave a caller that asked for
    /// three ingredients holding one and no way to know.
    ///
    /// # Errors
    ///
    /// [`DomainError::Insufficient`] if fewer than `quantity` are held.
    pub fn remove(&mut self, item: &ItemId, quantity: u32) -> Result<(), DomainError> {
        let held = self.count(item);
        if held < quantity {
            return Err(DomainError::Insufficient {
                resource: "items",
                needed: i64::from(quantity),
                available: i64::from(held),
            });
        }
        let remaining = held - quantity;
        if remaining == 0 {
            // An empty slot is freed rather than kept at zero, so `count` has
            // exactly one representation of "none".
            self.slots.remove(item);
        } else {
            self.slots.insert(item.clone(), remaining);
        }
        Ok(())
    }
}

impl Default for Inventory {
    fn default() -> Self {
        Self::with_capacity(Self::DEFAULT_CAPACITY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(name: &str) -> ItemId {
        ItemId::new(name).expect("valid item id")
    }

    #[test]
    fn adding_and_counting_round_trip() {
        let mut inventory = Inventory::default();
        inventory.add(item("berry"), 3).expect("add");
        inventory.add(item("berry"), 2).expect("top up");
        assert_eq!(inventory.count(&item("berry")), 5);
        assert_eq!(inventory.distinct_kinds(), 1);
    }

    #[test]
    fn an_absent_item_counts_zero() {
        assert_eq!(Inventory::default().count(&item("berry")), 0);
    }

    #[test]
    fn removing_everything_frees_the_slot() {
        let mut inventory = Inventory::default();
        inventory.add(item("berry"), 2).expect("add");
        inventory.remove(&item("berry"), 2).expect("remove");
        assert_eq!(
            inventory.distinct_kinds(),
            0,
            "a spent slot must be released"
        );
        assert!(inventory.is_empty());
    }

    #[test]
    fn removing_more_than_held_fails_without_partial_effect() {
        let mut inventory = Inventory::default();
        inventory.add(item("berry"), 2).expect("add");

        let error = inventory.remove(&item("berry"), 5).unwrap_err();
        assert!(matches!(error, DomainError::Insufficient { .. }));
        assert_eq!(
            inventory.count(&item("berry")),
            2,
            "a failed removal must change nothing"
        );
    }

    #[test]
    fn capacity_limits_distinct_kinds_only() {
        let mut inventory = Inventory::with_capacity(2);
        inventory.add(item("berry"), 1).expect("first kind");
        inventory.add(item("potion"), 1).expect("second kind");
        assert!(inventory.is_full());

        let error = inventory.add(item("stick"), 1).unwrap_err();
        assert!(matches!(error, DomainError::AtCapacity { .. }));

        inventory
            .add(item("berry"), 100)
            .expect("topping up a held kind must still work");
        assert_eq!(inventory.count(&item("berry")), 101);
    }

    #[test]
    fn adding_zero_is_a_no_op_even_at_capacity() {
        let mut inventory = Inventory::with_capacity(0);
        inventory
            .add(item("berry"), 0)
            .expect("zero must not need a slot");
        assert!(inventory.is_empty());
    }

    #[test]
    fn quantities_saturate_rather_than_wrapping() {
        let mut inventory = Inventory::default();
        inventory.add(item("berry"), u32::MAX).expect("add");
        inventory.add(item("berry"), 10).expect("add");
        assert_eq!(inventory.count(&item("berry")), u32::MAX);
    }

    #[test]
    fn iteration_is_ordered_by_item_id() {
        let mut inventory = Inventory::default();
        for name in ["zinc", "apple", "mango"] {
            inventory.add(item(name), 1).expect("add");
        }
        let order: Vec<&str> = inventory.iter().map(|(id, _)| id.as_str()).collect();
        assert_eq!(order, ["apple", "mango", "zinc"]);
    }
}
