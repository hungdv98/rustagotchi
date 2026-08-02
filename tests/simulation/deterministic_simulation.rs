//! The same seed must produce byte-identical state.
//!
//! Two sections. The first runs today and covers the determinism primitives the
//! whole design rests on. The second is the Phase 3 contract, `#[ignore]`d until
//! there is a simulation to point it at.

use domain::rng::{RngStream, WorldSeed, rng_for};
use domain::time::{GameClock, GameDuration, TICKS_PER_DAY, Tick, UnixSeconds};
use domain::units::{Fixed, Milli};
use rustagotchi_tests::harness::{PendingSimulation, SimulationHarness};
use test_support::seeded_rng::TEST_SEED;

// ---------------------------------------------------------------------------
// Runnable today: the primitives.
// ---------------------------------------------------------------------------

/// Ten thousand ticks of derived randomness, twice, compared exactly.
///
/// If this fails, nothing downstream can be deterministic.
#[test]
fn derived_randomness_is_identical_across_runs() {
    fn sample_ten_thousand_ticks() -> Vec<u64> {
        (0..10_000)
            .map(|tick| {
                let mut rng = rng_for(TEST_SEED, RngStream::NeedsDecay, Tick(tick), 1);
                rng.next_u64()
            })
            .collect()
    }

    assert_eq!(sample_ten_thousand_ticks(), sample_ten_thousand_ticks());
}

/// Order of traversal must not matter.
///
/// A real simulation visits pets in some order every tick. Because each pet's
/// randomness is derived from its own key rather than drawn from a shared
/// stream, visiting them in a different order cannot change any outcome — which
/// is what lets the scheduler be reordered without rewriting history.
#[test]
fn traversal_order_does_not_affect_any_entitys_draws() {
    let entities: Vec<u64> = (1..=32).collect();

    let forward: Vec<(u64, u64)> = entities
        .iter()
        .map(|entity| {
            let mut rng = rng_for(TEST_SEED, RngStream::IllnessOnset, Tick(500), *entity);
            (*entity, rng.next_u64())
        })
        .collect();

    let mut backward: Vec<(u64, u64)> = entities
        .iter()
        .rev()
        .map(|entity| {
            let mut rng = rng_for(TEST_SEED, RngStream::IllnessOnset, Tick(500), *entity);
            (*entity, rng.next_u64())
        })
        .collect();
    backward.reverse();

    assert_eq!(forward, backward);
}

/// Adding a roll to one system must not perturb another.
///
/// This is the property that keeps combat balance changes from silently
/// rewriting which pets mutate (ADR 0007).
#[test]
fn consuming_extra_draws_in_one_stream_does_not_disturb_another() {
    let mutation_before = {
        let mut rng = rng_for(TEST_SEED, RngStream::Mutation, Tick(42), 7);
        rng.next_u64()
    };

    // Simulate a balance change that adds three accuracy rolls to combat.
    let mut combat = rng_for(TEST_SEED, RngStream::CombatHit, Tick(42), 7);
    for _ in 0..3 {
        let _ = combat.next_u64();
    }

    let mutation_after = {
        let mut rng = rng_for(TEST_SEED, RngStream::Mutation, Tick(42), 7);
        rng.next_u64()
    };

    assert_eq!(mutation_before, mutation_after);
}

/// A year of clock advancement, driven two different ways, must agree.
#[test]
fn a_year_of_clock_advancement_is_path_independent() {
    let year = TICKS_PER_DAY * 365;

    let mut one_step = GameClock::starting_at(UnixSeconds(0));
    one_step.advance(GameDuration::from_ticks(year));

    let mut daily = GameClock::starting_at(UnixSeconds(0));
    for _ in 0..365 {
        daily.advance(GameDuration::from_days(1));
    }

    assert_eq!(one_step.current(), daily.current());
    assert_eq!(one_step.current(), Tick(year));
    assert_eq!(one_step.anchored_at(), daily.anchored_at());
}

/// Integer decay accumulated over a year must be exactly reproducible, and must
/// not drift between an aggregate and a per-tick application.
#[test]
fn a_year_of_integer_decay_does_not_drift() {
    let per_tick = Milli(10); // a full 100.000 bar drains in 10,000 ticks
    let ticks = 525_600i64;

    let mut accumulated = Milli::ZERO;
    for _ in 0..ticks {
        accumulated = accumulated.saturating_add(per_tick);
    }

    // Saturation is the only bound; there is no rounding error to accumulate.
    assert_eq!(accumulated, Milli((ticks * per_tick.0 as i64) as i32));
}

/// Applying a rate tick by tick is *not* the same as applying it once to the
/// total, and the simulation must be written knowing which one it means.
///
/// Recorded as a test rather than a comment because it is the single easiest way
/// to introduce a live/offline divergence: apply the rate per tick in one path
/// and to the aggregate in the other.
#[test]
fn per_tick_and_aggregate_rate_application_differ_by_truncation() {
    let rate = Fixed::from_ratio(1, 3);
    let per_tick_value = Milli(10);

    let stepwise: i32 = (0..100).map(|_| rate.apply(per_tick_value).0).sum();
    let aggregate = rate.apply(Milli(per_tick_value.0 * 100)).0;

    assert_eq!(stepwise, 300, "each tick truncates 3.333 down to 3");
    assert_eq!(aggregate, 333, "the aggregate truncates only once");
    assert_ne!(
        stepwise, aggregate,
        "offline catch-up must therefore step tick by tick, per ADR 0003"
    );
}

// ---------------------------------------------------------------------------
// Phase 3 contract.
// ---------------------------------------------------------------------------

#[test]
fn the_same_seed_produces_identical_state_after_ten_thousand_ticks() {
    let run = || {
        let world = PendingSimulation::new_world(WorldSeed(12_345));
        PendingSimulation::run_live(world, 10_000)
    };
    assert_eq!(run(), run());
}

#[test]
fn different_seeds_produce_different_state() {
    let first = PendingSimulation::run_live(PendingSimulation::new_world(WorldSeed(1)), 10_000);
    let second = PendingSimulation::run_live(PendingSimulation::new_world(WorldSeed(2)), 10_000);
    assert_ne!(first, second, "distinct worlds must actually diverge");
}
