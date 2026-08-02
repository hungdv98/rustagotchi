use std::path::Path;

use content::ContentLoader;
use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use domain::genetics::{Ancestry, GeneticProfile, Parentage};
use domain::identifiers::{PetId, SpeciesId};
use domain::pet::{Personality, Pet};
use domain::rng::WorldSeed;
use domain::time::Tick;
use simulation::{SimConfig, SimulationState, fixed_step};

fn benchmark(c: &mut Criterion) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    let content = ContentLoader::load(&root).unwrap();
    let config =
        SimConfig::from_ron(&std::fs::read_to_string(root.join("config/simulation.ron")).unwrap())
            .unwrap();
    let seed = WorldSeed(1);
    let mut state = SimulationState::new(seed);
    for ordinal in 0..8 {
        state.insert_pet(Pet::hatch(
            PetId::derive(seed, Tick::ZERO, ordinal),
            SpeciesId::new("emberling").unwrap(),
            GeneticProfile::new(),
            Personality::neutral(),
            Parentage::Founder,
            Ancestry::founder(),
            Tick::ZERO,
        ));
    }
    c.bench_function("fixed_step_8_pets", |b| {
        b.iter_batched(
            || state.clone(),
            |mut sample| fixed_step::run_one(&mut sample, &content, &config),
            BatchSize::SmallInput,
        )
    });
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
