use std::collections::BTreeMap;

use domain::breeding::{
    BreedingEligibilityError, BreedingRules, Egg, IncubationState, validate_pair,
};
use domain::genetics::{Allele, Ancestry, GenePair, GeneticProfile, Mutation, MutationRate};
use domain::identifiers::{EggId, GeneId, SpeciesId};
use domain::pet::Pet;
use domain::rng::{RngStream, WorldSeed, rng_for};
use domain::time::{GameDuration, Tick};
use domain::units::Milli;

#[derive(Debug, Clone)]
pub struct OffspringConfig {
    pub eligibility: BreedingRules,
    pub incubation: GameDuration,
    pub mutation_rate: MutationRate,
    pub mutation_candidates: BTreeMap<GeneId, Vec<Allele>>,
}

impl Default for OffspringConfig {
    fn default() -> Self {
        Self {
            eligibility: BreedingRules::default(),
            incubation: GameDuration::from_days(2),
            mutation_rate: MutationRate::NEVER,
            mutation_candidates: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BreedingError {
    #[error(transparent)]
    Ineligible(#[from] BreedingEligibilityError),
    #[error("the species pairing is incompatible")]
    Incompatible,
    #[error("parent genetic profiles have different loci")]
    GeneticSchemaMismatch,
}

#[allow(clippy::too_many_arguments)]
pub fn generate_egg(
    first: &Pet,
    second: &Pet,
    species: SpeciesId,
    compatibility: Milli,
    seed: WorldSeed,
    now: Tick,
    ordinal: u64,
    last_bred: [Option<Tick>; 2],
    config: &OffspringConfig,
) -> Result<Egg, BreedingError> {
    validate_pair(first, second, now, last_bred, config.eligibility)?;
    let id = EggId::derive(seed, now, ordinal);
    let mut success_rng = rng_for(seed, RngStream::GeneInheritance, now, id.rng_key());
    if !success_rng.chance(compatibility.0.clamp(0, 100_000) as u32, 100_000) {
        return Err(BreedingError::Incompatible);
    }

    let first_loci: Vec<_> = first.genetics.iter().map(|(id, _)| id).collect();
    let second_loci: Vec<_> = second.genetics.iter().map(|(id, _)| id).collect();
    if first_loci != second_loci {
        return Err(BreedingError::GeneticSchemaMismatch);
    }

    let mut inheritance_rng = rng_for(seed, RngStream::GeneInheritance, now, id.rng_key() ^ 1);
    let mut mutation_rng = rng_for(seed, RngStream::Mutation, now, id.rng_key());
    let mut genetics = GeneticProfile::new();
    let mut mutations = Vec::new();
    for (gene, first_pair) in first.genetics.iter() {
        let second_pair = second.genetics.pair(gene).expect("schemas checked");
        let mut inherited = [
            first_pair.sorted_alleles()[inheritance_rng.below(2) as usize],
            second_pair.sorted_alleles()[inheritance_rng.below(2) as usize],
        ];
        if let Some(candidates) = config.mutation_candidates.get(gene) {
            for allele in &mut inherited {
                if mutation_rng.chance(config.mutation_rate.numerator(), MutationRate::DENOMINATOR)
                {
                    let alternatives: Vec<_> = candidates
                        .iter()
                        .copied()
                        .filter(|candidate| candidate.symbol != allele.symbol)
                        .collect();
                    if let Some(replacement) = mutation_rng.pick(&alternatives).copied() {
                        mutations.push(Mutation::new(gene.clone(), allele.symbol, replacement));
                        *allele = replacement;
                    }
                }
            }
        }
        genetics.insert(gene.clone(), GenePair::new(inherited[0], inherited[1]));
    }

    Ok(Egg {
        id,
        parents: [first.id, second.id],
        species,
        genetics,
        ancestry: Ancestry::from_parents(first.id, &first.ancestry, second.id, &second.ancestry),
        incubation: IncubationState {
            laid_at: now,
            required: config.incubation,
        },
        mutations,
    })
}

#[cfg(test)]
mod tests {
    use domain::genetics::{Allele, GeneticProfile, Parentage};
    use domain::identifiers::PetId;
    use domain::pet::{LifeStage, Personality};

    use super::*;

    fn parent(ordinal: u64, alleles: [u16; 2]) -> Pet {
        let seed = WorldSeed(9);
        let gene = GeneId::new("coat_colour").unwrap();
        let mut pet = Pet::hatch(
            PetId::derive(seed, Tick::ZERO, ordinal),
            SpeciesId::new("emberling").unwrap(),
            GeneticProfile::from_loci([(
                gene,
                GenePair::new(Allele::standard(alleles[0]), Allele::standard(alleles[1])),
            )]),
            Personality::neutral(),
            Parentage::Founder,
            Ancestry::founder(),
            Tick::ZERO,
        );
        pet.stage = LifeStage::Adult;
        pet
    }

    #[test]
    fn generation_is_reproducible_and_records_lineage() {
        let first = parent(1, [1, 2]);
        let second = parent(2, [3, 4]);
        let create = || {
            generate_egg(
                &first,
                &second,
                SpeciesId::new("emberling").unwrap(),
                Milli(100_000),
                WorldSeed(9),
                Tick(20_000),
                3,
                [None, None],
                &OffspringConfig::default(),
            )
            .unwrap()
        };
        let egg = create();
        assert_eq!(egg, create());
        assert_eq!(egg.parents, [first.id, second.id]);
        assert!(egg.ancestry.is_ancestor(first.id));
    }

    #[test]
    fn certain_mutation_uses_a_different_catalogued_allele() {
        let first = parent(1, [1, 1]);
        let second = parent(2, [1, 1]);
        let mut config = OffspringConfig {
            mutation_rate: MutationRate::per_million(1_000_000).unwrap(),
            ..OffspringConfig::default()
        };
        config.mutation_candidates.insert(
            GeneId::new("coat_colour").unwrap(),
            vec![Allele::standard(1), Allele::standard(9)],
        );
        let egg = generate_egg(
            &first,
            &second,
            SpeciesId::new("emberling").unwrap(),
            Milli(100_000),
            WorldSeed(9),
            Tick(20_000),
            3,
            [None, None],
            &config,
        )
        .unwrap();
        assert_eq!(egg.mutations.len(), 2);
        assert!(egg.genetics.carries(Allele::standard(9)));
    }
}
