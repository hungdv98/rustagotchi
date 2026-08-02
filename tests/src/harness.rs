//! The simulation surface the determinism suites are written against.
//!
//! Phase 3 satisfies this contract by replacing [`PendingSimulation`] with a
//! wrapper over `simulation::fixed_step`. Until then the tests that use it are
//! `#[ignore]`d — they compile, so the shape is locked in, and they panic with
//! "not yet implemented" rather than passing vacuously if anyone un-ignores one
//! early.

use std::path::Path;
use std::sync::OnceLock;

use domain::genetics::{Ancestry, GeneticProfile, Parentage};
use domain::identifiers::{PetId, SpeciesId};
use domain::pet::{Personality, Pet};
use domain::rng::WorldSeed;
use domain::time::{GameDuration, Tick};
use simulation::{SimConfig, SimulationState};

/// Stand-in for the state Phase 4's `application::GameState` will provide.
///
/// `PartialEq` is the whole point: offline/live equivalence and save/load
/// equivalence are both exact equality of this value, which is only possible
/// because ADR 0006 keeps every simulated quantity an integer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldSnapshot {
    pub tick: Tick,
    /// A cheap structural digest, for asserting divergence in long runs without
    /// holding two full states in memory.
    pub fingerprint: u64,
    state: SimulationState,
}

/// What the simulation must offer for these suites to run.
pub trait SimulationHarness {
    /// A fresh world at [`Tick::ZERO`].
    fn new_world(seed: WorldSeed) -> WorldSnapshot;

    /// Advances tick by tick, as the live game loop does.
    fn run_live(world: WorldSnapshot, ticks: u64) -> WorldSnapshot;

    /// Catches up after an absence.
    ///
    /// ADR 0003 requires this to be the *same* code path as [`Self::run_live`],
    /// not an approximation of it. `offline_equivalence` is what holds it to
    /// that; if this ever becomes a separate closed-form implementation, the
    /// test stops being possible to write.
    fn catch_up_offline(world: WorldSnapshot, elapsed: GameDuration) -> WorldSnapshot;
}

#[derive(Debug)]
pub struct PendingSimulation;

impl SimulationHarness for PendingSimulation {
    fn new_world(seed: WorldSeed) -> WorldSnapshot {
        let mut state = SimulationState::new(seed);
        let pet = Pet::hatch(
            PetId::derive(seed, Tick::ZERO, 1),
            SpeciesId::new("emberling").unwrap(),
            GeneticProfile::new(),
            Personality::neutral(),
            Parentage::Founder,
            Ancestry::founder(),
            Tick::ZERO,
        );
        state.insert_pet(pet);
        snapshot(state)
    }

    fn run_live(mut world: WorldSnapshot, ticks: u64) -> WorldSnapshot {
        for _ in 0..ticks {
            simulation::fixed_step::run_one(&mut world.state, content(), config());
        }
        snapshot(world.state)
    }

    fn catch_up_offline(mut world: WorldSnapshot, elapsed: GameDuration) -> WorldSnapshot {
        simulation::offline::catch_up(&mut world.state, elapsed, content(), config());
        snapshot(world.state)
    }
}

fn content() -> &'static content::ContentRegistry {
    static CONTENT: OnceLock<content::ContentRegistry> = OnceLock::new();
    CONTENT.get_or_init(|| {
        content::ContentLoader::load(Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets"))
            .unwrap()
    })
}

fn config() -> &'static SimConfig {
    static CONFIG: OnceLock<SimConfig> = OnceLock::new();
    CONFIG.get_or_init(|| {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/config/simulation.ron");
        SimConfig::from_ron(&std::fs::read_to_string(path).unwrap()).unwrap()
    })
}

fn snapshot(state: SimulationState) -> WorldSnapshot {
    let mut fingerprint = state.seed.0 ^ state.tick.0.rotate_left(17);
    for pet in state.pets.values() {
        fingerprint = fingerprint.rotate_left(7) ^ pet.id.rng_key();
        fingerprint ^= u64::from(pet.progression.level().get()).rotate_left(29);
        fingerprint ^= pet.species.as_str().bytes().fold(0u64, |value, byte| {
            value.wrapping_mul(257).wrapping_add(u64::from(byte))
        });
        for (_, need) in pet.needs.iter() {
            fingerprint = fingerprint.rotate_left(3) ^ need.get().0 as u64;
        }
        fingerprint ^= pet.health.current().0 as u64;
    }
    WorldSnapshot {
        tick: state.tick,
        fingerprint,
        state,
    }
}
