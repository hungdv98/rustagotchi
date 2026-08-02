use std::collections::BTreeMap;
use std::fmt::Write;

use domain::combat::{BattleAction, BattleState, Combatant, Element};
use domain::genetics::{Allele, Ancestry, GenePair, GeneticProfile, Parentage};
use domain::identifiers::{BattleId, GeneId, MoveId, PetId, SpeciesId};
use domain::pet::{LifeStage, Personality, Pet, Stats};
use domain::rng::{RngStream, WorldSeed, rng_for};
use domain::time::Tick;
use domain::units::Milli;

pub(crate) fn run(
    offspring: Option<u64>,
    battles: Option<u64>,
    assets: &std::path::Path,
) -> Result<String, String> {
    if offspring.is_none() && battles.is_none() {
        return Err("provide --offspring, --battles, or both".into());
    }
    let content = content::ContentLoader::load(assets).map_err(|error| error.to_string())?;
    let mut summary = String::new();
    if let Some(count) = offspring {
        benchmark_offspring(count, &mut summary)?;
    }
    if let Some(count) = battles {
        benchmark_battles(count, &content, &mut summary)?;
    }
    Ok(summary.trim_end().to_owned())
}

fn benchmark_offspring(count: u64, summary: &mut String) -> Result<(), String> {
    if count == 0 {
        return Err("offspring count must be positive".into());
    }
    let seed = WorldSeed(7_001);
    let now = Tick(20_000);
    let gene = GeneId::new("coat_colour").map_err(|error| error.to_string())?;
    let first = parent(seed, 1, &gene, [1, 2]);
    let second = parent(seed, 2, &gene, [3, 4]);
    let mut first_allele = 0u64;
    for ordinal in 1..=count {
        let egg = simulation::breeding::generate_egg(
            &first,
            &second,
            SpeciesId::new("emberling").map_err(|error| error.to_string())?,
            Milli(100_000),
            seed,
            now,
            ordinal,
            [None, None],
            &simulation::breeding::OffspringConfig::default(),
        )
        .map_err(|error| error.to_string())?;
        if egg
            .genetics
            .pair(&gene)
            .expect("inherited locus")
            .maternal
            .symbol
            .0
            == 1
        {
            first_allele += 1;
        }
    }
    let permyriad = first_allele.saturating_mul(10_000) / count;
    if count >= 10_000 && !(4_800..=5_200).contains(&permyriad) {
        return Err(format!(
            "inheritance distribution out of tolerance: {}.{:02}%",
            permyriad / 100,
            permyriad % 100
        ));
    }
    writeln!(
        summary,
        "offspring: {count}, allele split={}.{:02}%/{}.{:02}%",
        permyriad / 100,
        permyriad % 100,
        (10_000 - permyriad) / 100,
        (10_000 - permyriad) % 100
    )
    .expect("writing to String cannot fail");
    Ok(())
}

fn benchmark_battles(
    count: u64,
    content: &content::ContentRegistry,
    summary: &mut String,
) -> Result<(), String> {
    if count == 0 {
        return Err("battle count must be positive".into());
    }
    let moves = [
        MoveId::new("tackle").map_err(|error| error.to_string())?,
        MoveId::new("warm_nuzzle").map_err(|error| error.to_string())?,
        MoveId::new("static_puff").map_err(|error| error.to_string())?,
    ];
    let mut usage = BTreeMap::<String, u64>::new();
    let mut first_wins = 0u64;
    for index in 0..count {
        let seed = WorldSeed(80_000 + index);
        let first = PetId::derive(seed, Tick::ZERO, 1);
        let second = PetId::derive(seed, Tick::ZERO, 2);
        let state = BattleState {
            id: BattleId::derive(seed, Tick::ZERO, 3),
            combatants: [
                Combatant::from_stats(first, Element::Ember, Stats::uniform(10)),
                Combatant::from_stats(second, Element::Ember, Stats::uniform(10)),
            ],
            turn: 0,
        };
        let actions = (0..12)
            .map(|turn| {
                [first, second].map(|actor| {
                    let mut rng = rng_for(seed, RngStream::CombatAi, Tick(turn), actor.rng_key());
                    let move_id = moves[rng.below(moves.len() as u64) as usize].clone();
                    *usage.entry(move_id.to_string()).or_default() += 1;
                    BattleAction { actor, move_id }
                })
            })
            .collect::<Vec<_>>();
        let result = simulation::combat::resolve_battle(seed, state, &actions, content)
            .map_err(|error| format!("battle {index}: {error}"))?;
        first_wins += u64::from(result.winner == first);
    }
    let win_permyriad = first_wins.saturating_mul(10_000) / count;
    if count >= 10_000 && !(4_500..=5_500).contains(&win_permyriad) {
        return Err(format!(
            "mirror win rate out of tolerance: {}.{:02}%",
            win_permyriad / 100,
            win_permyriad % 100
        ));
    }
    let total_usage: u64 = usage.values().sum();
    if count >= 1_000 {
        for (move_id, used) in &usage {
            let percent = used.saturating_mul(100) / total_usage;
            if !(5..=70).contains(&percent) {
                return Err(format!("move `{move_id}` usage is {percent}%"));
            }
        }
    }
    writeln!(
        summary,
        "battles: {count}, first-player wins={}.{:02}%, move usage={usage:?}",
        win_permyriad / 100,
        win_permyriad % 100
    )
    .expect("writing to String cannot fail");
    Ok(())
}

fn parent(seed: WorldSeed, ordinal: u64, gene: &GeneId, alleles: [u16; 2]) -> Pet {
    let mut pet = Pet::hatch(
        PetId::derive(seed, Tick::ZERO, ordinal),
        SpeciesId::new("emberling").expect("base content id"),
        GeneticProfile::from_loci([(
            gene.clone(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_scale_drivers_execute() {
        let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let summary = run(Some(100), Some(100), &assets).unwrap();
        assert!(summary.contains("offspring: 100"));
        assert!(summary.contains("battles: 100"));
    }
}
