//! Items the player holds and the pet wears.
//!
//! What an item *does* — how much hunger a food restores, which illness a
//! medicine cures — is content (ADR 0005). This module owns only the rules of
//! holding, stacking and wearing.

#[allow(clippy::module_inception)]
pub mod inventory;

pub mod equipment;
pub mod item;
pub mod stack;

pub use equipment::Equipment;
pub use inventory::Inventory;
pub use item::{EquipmentSlot, ItemCategory};
pub use stack::ItemStack;
