use serde::{Deserialize, Serialize};

use crate::genetics::{Ancestry, GeneticProfile, Mutation};
use crate::identifiers::{EggId, PetId, SpeciesId};
use crate::time::{GameDuration, Tick};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct IncubationState {
    pub laid_at: Tick,
    pub required: GameDuration,
}

impl IncubationState {
    #[must_use]
    pub const fn elapsed(self, now: Tick) -> GameDuration {
        now.since(self.laid_at)
    }

    #[must_use]
    pub const fn remaining(self, now: Tick) -> GameDuration {
        self.required.saturating_sub(self.elapsed(now))
    }

    #[must_use]
    pub const fn is_ready(self, now: Tick) -> bool {
        self.elapsed(now).as_ticks() >= self.required.as_ticks()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Egg {
    pub id: EggId,
    pub parents: [PetId; 2],
    pub species: SpeciesId,
    pub genetics: GeneticProfile,
    pub ancestry: Ancestry,
    pub incubation: IncubationState,
    pub mutations: Vec<Mutation>,
}

impl Egg {
    #[must_use]
    pub fn is_ready(&self, now: Tick) -> bool {
        self.incubation.is_ready(now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::WorldSeed;

    #[test]
    fn readiness_changes_at_the_exact_incubation_tick() {
        let state = IncubationState {
            laid_at: Tick(100),
            required: GameDuration::from_days(2),
        };
        assert!(!state.is_ready(Tick(100 + GameDuration::from_days(2).as_ticks() - 1)));
        assert!(state.is_ready(Tick(100 + GameDuration::from_days(2).as_ticks())));
        let _ = EggId::derive(WorldSeed(1), Tick::ZERO, 1);
    }
}
