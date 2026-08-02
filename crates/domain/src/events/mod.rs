//! Domain events — the only way information leaves the core. See ADR 0009.

pub mod breeding_event;
pub mod domain_event;
pub mod pet_event;

pub use breeding_event::BreedingEvent;
pub use domain_event::{DomainEvent, DomainEvents};
pub use pet_event::{DeathCause, PetEvent};
