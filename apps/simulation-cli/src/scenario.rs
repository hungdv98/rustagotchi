use application::{Command, GameEngine, GameState};
use domain::combat::BattleAction;
use domain::genetics::{Ancestry, GeneticProfile, Parentage};
use domain::identifiers::{ItemId, MoveId, PetId, SaveSlotId, SpeciesId};
use domain::pet::{LifeStage, NeedKind, NeedValue, Personality, Pet};
use domain::rng::WorldSeed;
use domain::time::{GameDuration, Tick, UnixSeconds};
use domain::units::Milli;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VerticalSlice {
    seed: u64,
    initial_age_days: u64,
    advance_days: u64,
    feed_item: String,
    battle_move: String,
}

pub(crate) fn run(script: &std::path::Path, assets: &std::path::Path) -> Result<String, String> {
    let source = std::fs::read_to_string(script)
        .map_err(|error| format!("cannot read {}: {error}", script.display()))?;
    let script: VerticalSlice =
        ron::from_str(&source).map_err(|error| format!("invalid scenario: {error}"))?;
    let content = content::ContentLoader::load(assets).map_err(|error| error.to_string())?;
    let config_source = std::fs::read_to_string(assets.join("config/simulation.ron"))
        .map_err(|error| error.to_string())?;
    let mut config =
        simulation::SimConfig::from_ron(&config_source).map_err(|error| error.to_string())?;
    // The scenario validates orchestration, not neglect balancing. Pausing
    // passive decay keeps its single explicit feed meaningful across the
    // scripted incubation window.
    config.need_decay_per_tick.hunger = 0;
    config.need_decay_per_tick.energy = 0;
    config.need_decay_per_tick.hygiene = 0;
    config.need_decay_per_tick.bladder = 0;
    config.need_decay_per_tick.social = 0;
    config.need_decay_per_tick.fun = 0;
    config.illness_roll_numerator_per_day = 0;
    let temporary = tempfile::tempdir().map_err(|error| error.to_string())?;
    let repository = persistence::SqliteSaveRepository::new(temporary.path().join("slice.db"))
        .map_err(|error| error.to_string())?;
    let seed = WorldSeed(script.seed);
    let baseline = GameDuration::from_days(script.initial_age_days);
    let mut state = GameState::new(seed, UnixSeconds::EPOCH);
    state.simulation.tick = Tick(baseline.as_ticks());
    state.clock.advance(baseline);
    let parents = [
        PetId::derive(seed, Tick::ZERO, 1),
        PetId::derive(seed, Tick::ZERO, 2),
    ];
    for parent in parents {
        let mut pet = Pet::hatch(
            parent,
            SpeciesId::new("emberling").map_err(|error| error.to_string())?,
            GeneticProfile::new(),
            Personality::neutral(),
            Parentage::Founder,
            Ancestry::founder(),
            Tick::ZERO,
        );
        pet.stage = LifeStage::Adult;
        state.simulation.insert_pet(pet);
    }
    let opponent = PetId::derive(seed, Tick::ZERO, 3);
    state.simulation.insert_pet(Pet::hatch(
        opponent,
        SpeciesId::new("emberling").map_err(|error| error.to_string())?,
        GeneticProfile::new(),
        Personality::neutral(),
        Parentage::Founder,
        Ancestry::founder(),
        Tick::ZERO,
    ));
    state.next_entity_ordinal = 4;
    state
        .simulation
        .pets
        .get_mut(&parents[0])
        .expect("inserted")
        .set_need(
            NeedKind::Hunger,
            NeedValue::new(Milli(80_000)),
            state.simulation.tick,
        );
    let food = ItemId::new(&script.feed_item).map_err(|error| error.to_string())?;
    state
        .inventory
        .add(food.clone(), 1)
        .map_err(|error| error.to_string())?;
    let mut engine = GameEngine::new(state, content, config, repository);
    engine
        .dispatch(Command::FeedPet {
            pet: parents[0],
            item: food,
        })
        .map_err(|error| error.to_string())?;
    let advance = GameDuration::from_days(script.advance_days);
    engine
        .advance_to(UnixSeconds(
            (baseline.as_ticks() + advance.as_ticks()).saturating_mul(60) as i64,
        ))
        .map_err(|error| error.to_string())?;
    engine
        .dispatch(Command::Breed {
            first: parents[0],
            second: parents[1],
        })
        .map_err(|error| error.to_string())?;
    let egg = *engine
        .state()
        .eggs
        .keys()
        .next()
        .ok_or("breeding produced no egg")?;
    let incubation = engine.state().eggs[&egg].incubation.required;
    let hatch_wall = engine.state().clock.anchored_at().saturating_add_secs(
        incubation
            .as_ticks()
            .saturating_mul(60)
            .min(i64::MAX as u64) as i64,
    );
    engine
        .advance_to(hatch_wall)
        .map_err(|error| error.to_string())?;
    engine
        .dispatch(Command::HatchEgg { egg })
        .map_err(|error| error.to_string())?;
    let hatchling = engine
        .state()
        .simulation
        .pets
        .values()
        .find(|pet| matches!(pet.parentage, Parentage::Bred { .. }))
        .map(|pet| pet.id)
        .ok_or("egg did not hatch")?;
    engine
        .dispatch(Command::StartBattle {
            first: hatchling,
            second: opponent,
        })
        .map_err(|error| error.to_string())?;
    let battle = *engine
        .state()
        .active_battles
        .keys()
        .next()
        .ok_or("battle not created")?;
    let move_id = MoveId::new(&script.battle_move).map_err(|error| error.to_string())?;
    let actions = (0..10)
        .map(|_| {
            [
                BattleAction {
                    actor: hatchling,
                    move_id: move_id.clone(),
                },
                BattleAction {
                    actor: opponent,
                    move_id: move_id.clone(),
                },
            ]
        })
        .collect();
    engine
        .dispatch(Command::ResolveBattle { battle, actions })
        .map_err(|error| error.to_string())?;
    let slot = SaveSlotId::derive(seed, Tick::ZERO, 1);
    engine
        .dispatch(Command::Save { slot })
        .map_err(|error| error.to_string())?;
    let expected = engine.state().clone();
    engine
        .dispatch(Command::Load { slot })
        .map_err(|error| error.to_string())?;
    if engine.state() != &expected {
        return Err("save/load changed authoritative state".into());
    }
    engine.state().validate()?;
    Ok(format!(
        "vertical slice passed: tick={}, pets={}, coins={}, schema={}",
        engine.state().simulation.tick.0,
        engine.state().simulation.pets.len(),
        engine.state().wallet.balance().0,
        GameState::SCHEMA_VERSION
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn committed_vertical_slice_passes() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let summary = run(
            &root.join("tests/scripts/vertical_slice.ron"),
            &root.join("assets"),
        )
        .unwrap();
        assert!(summary.contains("vertical slice passed"));
        assert!(summary.contains("coins=50"));
    }
}
