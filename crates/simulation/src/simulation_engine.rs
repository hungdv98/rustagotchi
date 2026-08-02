//! Simulation state and its small orchestration wrapper.

use std::collections::BTreeMap;

use content::ContentRegistry;
use domain::events::DomainEvent;
use domain::identifiers::{PetId, SpeciesId};
use domain::pet::Pet;
use domain::rng::WorldSeed;
use domain::time::{GameDuration, Tick};
use serde::{Deserialize, Serialize};

use crate::{SimConfig, fixed_step};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimulationState {
    pub tick: Tick,
    pub seed: WorldSeed,
    pub pets: BTreeMap<PetId, Pet>,
    pub clamped_offline_ticks: u64,
}

impl SimulationState {
    #[must_use]
    pub fn new(seed: WorldSeed) -> Self {
        Self {
            tick: Tick::ZERO,
            seed,
            pets: BTreeMap::new(),
            clamped_offline_ticks: 0,
        }
    }
    pub fn insert_pet(&mut self, pet: Pet) -> Option<Pet> {
        self.pets.insert(pet.id, pet)
    }
    #[must_use]
    pub fn pet(&self, id: PetId) -> Option<&Pet> {
        self.pets.get(&id)
    }
    #[must_use]
    pub fn first_species(&self) -> Option<&SpeciesId> {
        self.pets.values().next().map(|pet| &pet.species)
    }
}

#[derive(Debug, Clone)]
pub struct SimulationEngine {
    state: SimulationState,
    config: SimConfig,
}

impl SimulationEngine {
    #[must_use]
    pub fn new(state: SimulationState, config: SimConfig) -> Self {
        Self { state, config }
    }
    #[must_use]
    pub fn state(&self) -> &SimulationState {
        &self.state
    }
    pub fn state_mut(&mut self) -> &mut SimulationState {
        &mut self.state
    }
    #[must_use]
    pub fn into_state(self) -> SimulationState {
        self.state
    }
    pub fn advance(
        &mut self,
        duration: GameDuration,
        content: &ContentRegistry,
    ) -> Vec<DomainEvent> {
        let mut events = Vec::new();
        for _ in 0..duration.as_ticks() {
            events.extend(fixed_step::run_one(&mut self.state, content, &self.config));
        }
        events
    }
}
