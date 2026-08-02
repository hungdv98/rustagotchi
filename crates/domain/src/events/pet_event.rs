//! Events emitted by the pet aggregate.

use serde::{Deserialize, Serialize};

use crate::identifiers::{IllnessId, PetId, SpeciesId};
use crate::pet::{LifeStage, NeedKind, NeedValue};
use crate::time::Tick;

/// Something that happened to a pet.
///
/// `#[non_exhaustive]` because consumers live in other crates — presentation,
/// telemetry, quests, achievements — and adding an event must not be a breaking
/// change for any of them (ADR 0009).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum PetEvent {
    /// An egg became a live pet.
    Hatched {
        pet: PetId,
        species: SpeciesId,
        at: Tick,
    },

    /// A player permanently removed a pet from the active world.
    Released { pet: PetId, at: Tick },

    /// A need moved. Carries both endpoints so a consumer can react to the
    /// *transition* — crossing a threshold, recovering — without keeping its own
    /// shadow copy of the previous value.
    NeedChanged {
        pet: PetId,
        need: NeedKind,
        from: NeedValue,
        to: NeedValue,
    },

    /// A need crossed into the critical band. Emitted once on entry, not every
    /// tick it stays there, so a notification does not become a siren.
    NeedBecameCritical {
        pet: PetId,
        need: NeedKind,
        at: Tick,
    },

    /// A need recovered out of the critical band.
    NeedRecovered {
        pet: PetId,
        need: NeedKind,
        at: Tick,
    },

    LifeStageAdvanced {
        pet: PetId,
        from: LifeStage,
        to: LifeStage,
        at: Tick,
    },

    Died {
        pet: PetId,
        cause: DeathCause,
        at: Tick,
    },
}

/// Why a pet died. Recorded in the event rather than derived later, because the
/// state that explains the death is gone by the time anything asks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum DeathCause {
    /// A need sat at zero for too long.
    Neglect {
        need: NeedKind,
    },
    Illness {
        illness: IllnessId,
    },
    OldAge,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::WorldSeed;

    fn pet() -> PetId {
        PetId::derive(WorldSeed(1), Tick(0), 0)
    }

    #[test]
    fn need_changes_carry_both_endpoints() {
        let event = PetEvent::NeedChanged {
            pet: pet(),
            need: NeedKind::Hunger,
            from: NeedValue::FULL,
            to: NeedValue::EMPTY,
        };
        let PetEvent::NeedChanged { from, to, .. } = event else {
            panic!("wrong variant");
        };
        assert_ne!(from, to);
    }

    #[test]
    fn events_survive_a_serde_round_trip() {
        let event = PetEvent::Died {
            pet: pet(),
            cause: DeathCause::Neglect {
                need: NeedKind::Hunger,
            },
            at: Tick(99),
        };
        let encoded = serde_json::to_string(&event).expect("serialize");
        let decoded: PetEvent = serde_json::from_str(&encoded).expect("deserialize");
        assert_eq!(event, decoded);
    }
}
