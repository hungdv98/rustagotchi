use std::path::Path;

use domain::genetics::{Ancestry, GeneticProfile, Parentage};
use domain::identifiers::{PetId, SpeciesId};
use domain::pet::{Personality, Pet};
use domain::rng::WorldSeed;
use domain::time::Tick;
use simulation::{SimConfig, SimulationState};

pub(crate) fn content() -> content::ContentRegistry {
    content::ContentLoader::load(Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets")).unwrap()
}

pub(crate) fn config() -> SimConfig {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/config/simulation.ron");
    SimConfig::from_ron(&std::fs::read_to_string(path).unwrap()).unwrap()
}

pub(crate) fn world(seed: WorldSeed) -> (SimulationState, PetId) {
    let mut state = SimulationState::new(seed);
    let id = PetId::derive(seed, Tick::ZERO, 1);
    state.insert_pet(Pet::hatch(
        id,
        SpeciesId::new("emberling").unwrap(),
        GeneticProfile::new(),
        Personality::neutral(),
        Parentage::Founder,
        Ancestry::founder(),
        Tick::ZERO,
    ));
    (state, id)
}
