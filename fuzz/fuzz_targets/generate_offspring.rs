#![no_main]

use domain::genetics::{Allele, Ancestry, GenePair, GeneticProfile, Parentage};
use domain::identifiers::{GeneId, PetId, SpeciesId};
use domain::pet::{LifeStage, Personality, Pet};
use domain::rng::WorldSeed;
use domain::time::Tick;
use domain::units::Milli;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() < 16 {
        return;
    }
    let seed = WorldSeed(u64::from_le_bytes(data[..8].try_into().unwrap()));
    let now = Tick(20_000);
    let gene = GeneId::new("fuzz_gene").unwrap();
    let parent = |ordinal, symbols: [u8; 2]| {
        let mut pet = Pet::hatch(
            PetId::derive(seed, Tick::ZERO, ordinal),
            SpeciesId::new("emberling").unwrap(),
            GeneticProfile::from_loci([(
                gene.clone(),
                GenePair::new(
                    Allele::standard(u16::from(symbols[0])),
                    Allele::standard(u16::from(symbols[1])),
                ),
            )]),
            Personality::neutral(),
            Parentage::Founder,
            Ancestry::founder(),
            Tick::ZERO,
        );
        pet.stage = LifeStage::Adult;
        pet
    };
    let first = parent(1, [data[8], data[9]]);
    let second = parent(2, [data[10], data[11]]);
    let compatibility = Milli(i32::from(data[12]) * 400);
    let _ = simulation::breeding::generate_egg(
        &first,
        &second,
        SpeciesId::new("emberling").unwrap(),
        compatibility,
        seed,
        now,
        u64::from_le_bytes(data[8..16].try_into().unwrap()),
        [None, None],
        &simulation::breeding::OffspringConfig::default(),
    );
});
