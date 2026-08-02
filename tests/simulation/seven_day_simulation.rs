mod support;

use domain::pet::{LifeStage, NeedKind, NeedValue};
use domain::rng::WorldSeed;
use domain::time::{GameDuration, TICKS_PER_DAY};
use simulation::SimulationEngine;

#[test]
fn a_neglected_pet_dies_within_seven_days() {
    let content = support::content();
    let config = simulation::SimConfig {
        illness_roll_numerator_per_day: 0,
        ..support::config()
    };
    let (state, id) = support::world(WorldSeed(7));
    let mut engine = SimulationEngine::new(state, config);
    engine.advance(GameDuration::from_days(7), &content);
    assert!(!engine.state().pet(id).unwrap().is_alive());
}

#[test]
fn an_hourly_cared_for_pet_grows_and_evolves() {
    let content = support::content();
    let config = simulation::SimConfig {
        illness_roll_numerator_per_day: 0,
        ..support::config()
    };
    let (state, id) = support::world(WorldSeed(8));
    let mut engine = SimulationEngine::new(state, config);
    for _ in 0..(TICKS_PER_DAY * 7 / 60) {
        engine.advance(GameDuration::from_hours(1), &content);
        let now = engine.state().tick;
        let pet = engine.state_mut().pets.get_mut(&id).unwrap();
        for kind in NeedKind::ALL {
            pet.set_need(kind, NeedValue::FULL, now);
        }
    }
    let pet = engine.state().pet(id).unwrap();
    assert!(pet.is_alive());
    assert_eq!(pet.stage, LifeStage::Adult);
    assert_ne!(pet.species.as_str(), "emberling");
}
