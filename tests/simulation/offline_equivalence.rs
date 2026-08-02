//! Catching up after an absence must equal having been there the whole time.
//!
//! Per docs/implementation-plan.md this is the highest-value test in the
//! repository, and it exists before the code it checks. ADR 0003 makes it
//! possible by refusing to build a separate approximate offline path: if
//! `catch_up_offline` is ever reimplemented as a closed-form shortcut, these
//! assertions stop holding and the build goes red.
//!
//! The first section runs today against the clock, which is the part of the
//! offline path that already exists. The second is the Phase 3 contract.

use domain::time::{GameClock, GameDuration, SECONDS_PER_TICK, TICKS_PER_DAY, Tick, UnixSeconds};
use proptest::prelude::*;
use rustagotchi_tests::harness::{PendingSimulation, SimulationHarness};
use test_support::seeded_rng::TEST_SEED;

// ---------------------------------------------------------------------------
// Runnable today: clock equivalence.
// ---------------------------------------------------------------------------

/// A week away and a week present must land on the same tick.
#[test]
fn a_week_offline_equals_a_week_of_polling() {
    let week_seconds = (TICKS_PER_DAY * 7 * SECONDS_PER_TICK) as i64;

    // Present the whole time: polled once a minute.
    let mut present = GameClock::starting_at(UnixSeconds(0));
    for second in (0..=week_seconds).step_by(SECONDS_PER_TICK as usize) {
        let owed = present.ticks_owed(UnixSeconds(second));
        present.advance(owed);
    }

    // Away: one poll on return.
    let mut away = GameClock::starting_at(UnixSeconds(0));
    let owed = away.ticks_owed(UnixSeconds(week_seconds));
    away.advance(owed);

    assert_eq!(present.current(), away.current());
    assert_eq!(present.current(), Tick(TICKS_PER_DAY * 7));
    assert_eq!(present.anchored_at(), away.anchored_at());
}

/// An absence longer than the cap is clamped, and says so.
///
/// Silently simulating less time than actually passed would leave a player
/// wondering why their pet is fine after three years away; recording the clamp
/// lets the UI explain it (ADR 0003).
#[test]
fn absences_beyond_the_cap_are_clamped_and_reported() {
    let cap = GameDuration::from_ticks(domain::time::MAX_OFFLINE_CATCHUP_TICKS);

    let (within, clamped) = GameDuration::from_days(30).clamped_to(cap);
    assert_eq!(within, GameDuration::from_days(30));
    assert!(!clamped);

    let (beyond, clamped) = GameDuration::from_days(3_000).clamped_to(cap);
    assert_eq!(beyond, cap);
    assert!(clamped, "the clamp must be reported, not silent");
}

proptest! {
    /// However the absence is split, the total game time is the same.
    #[test]
    fn splitting_an_absence_does_not_change_elapsed_time(
        splits in prop::collection::vec(1u64..5_000, 1..40),
    ) {
        let total_seconds: u64 = splits.iter().sum::<u64>() * SECONDS_PER_TICK;

        let mut piecewise = GameClock::starting_at(UnixSeconds(0));
        let mut wall = 0i64;
        for span in &splits {
            wall += (*span * SECONDS_PER_TICK) as i64;
            let owed = piecewise.ticks_owed(UnixSeconds(wall));
            piecewise.advance(owed);
        }

        let mut single = GameClock::starting_at(UnixSeconds(0));
        let owed = single.ticks_owed(UnixSeconds(total_seconds as i64));
        single.advance(owed);

        prop_assert_eq!(piecewise.current(), single.current());
    }
}

// ---------------------------------------------------------------------------
// Phase 3 contract.
// ---------------------------------------------------------------------------

#[test]
fn catching_up_a_day_equals_simulating_a_day() {
    let live = PendingSimulation::run_live(PendingSimulation::new_world(TEST_SEED), TICKS_PER_DAY);
    let offline = PendingSimulation::catch_up_offline(
        PendingSimulation::new_world(TEST_SEED),
        GameDuration::from_days(1),
    );
    assert_eq!(
        live, offline,
        "offline catch-up must be the live loop, not an approximation"
    );
}

#[test]
fn catching_up_a_year_equals_simulating_a_year() {
    let year = TICKS_PER_DAY * 365;
    let live = PendingSimulation::run_live(PendingSimulation::new_world(TEST_SEED), year);
    let offline = PendingSimulation::catch_up_offline(
        PendingSimulation::new_world(TEST_SEED),
        GameDuration::from_days(365),
    );
    assert_eq!(live, offline);
}

#[test]
fn an_interrupted_absence_equals_an_uninterrupted_one() {
    // Closing and reopening the game partway through must not change the result.
    let uninterrupted = PendingSimulation::catch_up_offline(
        PendingSimulation::new_world(TEST_SEED),
        GameDuration::from_days(10),
    );

    let mut interrupted = PendingSimulation::new_world(TEST_SEED);
    for _ in 0..10 {
        interrupted = PendingSimulation::catch_up_offline(interrupted, GameDuration::from_days(1));
    }

    assert_eq!(uninterrupted, interrupted);
}
