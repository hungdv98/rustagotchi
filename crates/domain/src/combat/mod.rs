//! Combat.
//!
//! Phase 5 builds this module out: `battle`, `combatant`, `action`,
//! `move_definition`, `move_effect`, `status_effect`, `damage`, `initiative`,
//! `rewards`, `result`.
//!
//! [`elemental`] arrived early, in Phase 2, because species and moves cannot be
//! authored without an element to give them.

mod battle;
pub mod elemental;
mod status_effect;

pub use battle::{BattleAction, BattleResult, BattleState, CombatEvent, Combatant};
pub use status_effect::{StatusEffect, StatusKind};

pub use elemental::Element;
