mod support;

use domain::rng::WorldSeed;
use domain::time::{GameDuration, TICKS_PER_DAY, Tick};

#[test]
fn a_year_catches_up_without_overflow_or_panics() {
    let content = support::content();
    let config = support::config();
    let (mut state, _) = support::world(WorldSeed(365));
    let report =
        simulation::offline::catch_up(&mut state, GameDuration::from_days(365), &content, &config);
    assert_eq!(report.simulated_ticks, TICKS_PER_DAY * 365);
    assert_eq!(report.clamped_ticks, 0);
    assert!(
        report.events.len() <= 4_096,
        "offline summaries must stay memory-bounded"
    );
    assert!(
        report.dropped_events > 0,
        "the report must disclose coalesced granular events"
    );
    assert_eq!(state.tick, Tick(TICKS_PER_DAY * 365));
    for pet in state.pets.values() {
        assert!(
            pet.needs
                .iter()
                .all(|(_, need)| (0..=100_000).contains(&need.get().0))
        );
        assert!((0..=100_000).contains(&pet.health.current().0));
    }
}
