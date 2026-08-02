//! Breeding invariants and saveable egg state.

mod egg;
mod eligibility;

pub use egg::{Egg, IncubationState};
pub use eligibility::{BreedingEligibilityError, BreedingRules, validate_pair};
