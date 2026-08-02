use application::GameState;
use domain::genetics::{Ancestry, GeneticProfile, Parentage};
use domain::identifiers::{ItemId, PetId, SaveSlotId, SpeciesId};
use domain::pet::{Personality, Pet};
use domain::rng::WorldSeed;
use domain::time::{Tick, UnixSeconds};

pub(crate) fn state() -> GameState {
    let seed = WorldSeed(77);
    let mut state = GameState::new(seed, UnixSeconds(1_000));
    state.simulation.insert_pet(Pet::hatch(
        PetId::derive(seed, Tick::ZERO, 1),
        SpeciesId::new("emberling").unwrap(),
        GeneticProfile::new(),
        Personality::neutral(),
        Parentage::Founder,
        Ancestry::founder(),
        Tick::ZERO,
    ));
    state
        .inventory
        .add(ItemId::new("apple_slice").unwrap(), 3)
        .unwrap();
    state
}
pub(crate) fn slot() -> SaveSlotId {
    SaveSlotId::derive(WorldSeed(77), Tick::ZERO, 1)
}
