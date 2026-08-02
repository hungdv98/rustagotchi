//! Quantities of one item.

use serde::{Deserialize, Serialize};

use crate::identifiers::ItemId;
use crate::validation::{ValidationError, require_range};

/// A quantity of a single item kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemStack {
    item: ItemId,
    quantity: u32,
}

impl ItemStack {
    /// The per-stack cap.
    ///
    /// A cap at all is what keeps arithmetic here bounded: without it a hostile
    /// or migrated save could carry a quantity that overflows when merged.
    pub const MAX_QUANTITY: u32 = 9_999;

    /// # Errors
    ///
    /// If `quantity` is zero or above [`Self::MAX_QUANTITY`]. A zero stack is
    /// not a valid value — absence is represented by the stack not existing, so
    /// there is exactly one way to say "none of these".
    pub fn new(item: ItemId, quantity: u32) -> Result<Self, ValidationError> {
        require_range(
            "item quantity",
            i64::from(quantity),
            1,
            i64::from(Self::MAX_QUANTITY),
        )?;
        Ok(ItemStack { item, quantity })
    }

    #[must_use]
    pub fn item(&self) -> &ItemId {
        &self.item
    }

    #[must_use]
    pub const fn quantity(&self) -> u32 {
        self.quantity
    }

    #[must_use]
    pub const fn is_full(&self) -> bool {
        self.quantity >= Self::MAX_QUANTITY
    }

    /// Adds up to `amount`, returning the remainder that did not fit.
    ///
    /// Returning the overflow rather than failing lets a caller spill into
    /// another stack, which is what a stacking inventory needs.
    pub const fn add(&mut self, amount: u32) -> u32 {
        let space = Self::MAX_QUANTITY - self.quantity;
        let accepted = if amount < space { amount } else { space };
        self.quantity += accepted;
        amount - accepted
    }

    /// Removes up to `amount`, returning how many were actually removed.
    pub const fn remove(&mut self, amount: u32) -> u32 {
        let removed = if amount < self.quantity {
            amount
        } else {
            self.quantity
        };
        self.quantity -= removed;
        removed
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.quantity == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item() -> ItemId {
        ItemId::new("berry_snack").expect("valid item id")
    }

    #[test]
    fn a_stack_holds_what_it_was_given() {
        let stack = ItemStack::new(item(), 5).expect("valid stack");
        assert_eq!(stack.quantity(), 5);
        assert_eq!(stack.item(), &item());
    }

    #[test]
    fn zero_and_oversized_stacks_are_rejected() {
        assert!(
            ItemStack::new(item(), 0).is_err(),
            "absence is not a stack of zero"
        );
        assert!(ItemStack::new(item(), ItemStack::MAX_QUANTITY + 1).is_err());
        assert!(ItemStack::new(item(), ItemStack::MAX_QUANTITY).is_ok());
    }

    #[test]
    fn adding_reports_what_did_not_fit() {
        let mut stack = ItemStack::new(item(), ItemStack::MAX_QUANTITY - 2).expect("valid");
        assert_eq!(stack.add(5), 3, "three should overflow");
        assert!(stack.is_full());
        assert_eq!(stack.quantity(), ItemStack::MAX_QUANTITY);
    }

    #[test]
    fn adding_within_capacity_overflows_nothing() {
        let mut stack = ItemStack::new(item(), 1).expect("valid");
        assert_eq!(stack.add(10), 0);
        assert_eq!(stack.quantity(), 11);
    }

    #[test]
    fn removing_more_than_present_takes_only_what_is_there() {
        let mut stack = ItemStack::new(item(), 3).expect("valid");
        assert_eq!(stack.remove(10), 3);
        assert!(stack.is_empty());
        assert_eq!(stack.remove(1), 0, "an empty stack yields nothing further");
    }

    #[test]
    fn quantities_cannot_overflow_on_a_hostile_input() {
        let mut stack = ItemStack::new(item(), 1).expect("valid");
        let overflow = stack.add(u32::MAX);
        assert_eq!(stack.quantity(), ItemStack::MAX_QUANTITY);
        assert_eq!(overflow, u32::MAX - (ItemStack::MAX_QUANTITY - 1));
    }
}
