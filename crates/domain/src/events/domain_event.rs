//! The single outward channel. See ADR 0009.

use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use super::BreedingEvent;
use super::pet_event::PetEvent;
use crate::combat::CombatEvent;

/// Anything the domain reports having happened.
///
/// Domain and simulation operations return these; presentation, audio,
/// telemetry, quests and achievements consume them. Gameplay code never calls
/// into any of those directly, which is what lets quests be added in Phase 7
/// without touching Phase 3 simulation code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum DomainEvent {
    WorldReset,
    Pet(PetEvent),
    Breeding(BreedingEvent),
    Combat(CombatEvent),
}

impl From<PetEvent> for DomainEvent {
    fn from(event: PetEvent) -> Self {
        DomainEvent::Pet(event)
    }
}

impl From<BreedingEvent> for DomainEvent {
    fn from(event: BreedingEvent) -> Self {
        DomainEvent::Breeding(event)
    }
}

impl From<CombatEvent> for DomainEvent {
    fn from(event: CombatEvent) -> Self {
        DomainEvent::Combat(event)
    }
}

/// The return type of a domain operation.
///
/// Inline capacity of four: the overwhelming majority of operations emit one or
/// two events, and a per-tick heap allocation per pet would eat a meaningful
/// slice of the one-microsecond budget in ADR 0003.
pub type DomainEvents = SmallVec<[DomainEvent; 4]>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identifiers::PetId;
    use crate::pet::{NeedKind, NeedValue};
    use crate::rng::WorldSeed;
    use crate::time::Tick;

    #[test]
    fn pet_events_convert_into_domain_events() {
        let event = PetEvent::NeedBecameCritical {
            pet: PetId::derive(WorldSeed(1), Tick(0), 0),
            need: NeedKind::Hunger,
            at: Tick(5),
        };
        let domain: DomainEvent = event.clone().into();
        assert_eq!(domain, DomainEvent::Pet(event));
    }

    #[test]
    fn the_common_case_stays_on_the_stack() {
        let mut events = DomainEvents::new();
        for index in 0..4 {
            events.push(DomainEvent::Pet(PetEvent::NeedChanged {
                pet: PetId::derive(WorldSeed(1), Tick(0), index),
                need: NeedKind::Fun,
                from: NeedValue::FULL,
                to: NeedValue::EMPTY,
            }));
        }
        assert!(!events.spilled(), "four events must not allocate");
        assert_eq!(events.len(), 4);
    }
}
