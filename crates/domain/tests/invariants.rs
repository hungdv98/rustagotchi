//! Property tests for the domain invariants.
//!
//! The unit tests next to each module check the cases we thought of. These check
//! the cases we did not — which, for arithmetic that has to stay exact across
//! 525,600 ticks and a save/load round trip, is where the bugs actually live.

use domain::economy::{Currency, Wallet};
use domain::events::{DeathCause, DomainEvent, PetEvent};
use domain::genetics::{Allele, Ancestry, GenePair, GeneticProfile, Parentage};
use domain::identifiers::{GeneId, ItemId, PetId, SpeciesId};
use domain::inventory::Inventory;
use domain::pet::{
    Activity, LifeStage, MemoryKind, NeedKind, NeedValue, Personality, Pet, PhysicalNeeds,
    Relationships, StatValue, Stats,
};
use domain::rng::{RngStream, WorldSeed, rng_for};
use domain::time::{GameClock, GameDuration, SECONDS_PER_TICK, Tick, UnixSeconds};
use domain::units::{Fixed, Milli};
use proptest::prelude::*;

fn any_milli() -> impl Strategy<Value = Milli> {
    any::<i32>().prop_map(Milli)
}

fn any_need() -> impl Strategy<Value = NeedValue> {
    any::<i32>().prop_map(|raw| NeedValue::new(Milli(raw)))
}

fn any_stage() -> impl Strategy<Value = LifeStage> {
    prop::sample::select(LifeStage::ORDER.to_vec())
}

fn any_needs() -> impl Strategy<Value = PhysicalNeeds> {
    prop::collection::vec(any_need(), 6).prop_map(|values| {
        let mut needs = PhysicalNeeds::full();
        for (kind, value) in NeedKind::ALL.into_iter().zip(values) {
            needs.set(kind, value);
        }
        needs
    })
}

// ---------------------------------------------------------------------------
// Invariant 1: a need is always within 0.000..=100.000, after any operation.
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn needs_are_always_in_range(raw in any::<i32>()) {
        let value = NeedValue::new(Milli(raw));
        prop_assert!(value.get() >= Milli::ZERO);
        prop_assert!(value.get() <= NeedValue::FULL.get());
    }

    #[test]
    fn draining_and_restoring_cannot_escape_the_range(
        start in any_need(),
        amount in any_milli(),
    ) {
        for result in [start.drained_by(amount), start.restored_by(amount)] {
            prop_assert!(result.get() >= Milli::ZERO);
            prop_assert!(result.get() <= NeedValue::FULL.get());
        }
    }

    #[test]
    fn restoring_never_decreases_and_draining_never_increases(
        start in any_need(),
        amount in 0i32..i32::MAX,
    ) {
        prop_assert!(start.restored_by(Milli(amount)) >= start);
        prop_assert!(start.drained_by(Milli(amount)) <= start);
    }

    #[test]
    fn the_lowest_need_really_is_the_lowest(needs in any_needs()) {
        let (kind, value) = needs.lowest();
        for (other_kind, other_value) in needs.iter() {
            prop_assert!(
                value <= other_value,
                "{kind} claimed lowest but {other_kind} is smaller"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Invariant 2: life stages never regress.
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn a_stage_only_ever_moves_forward(from in any_stage(), to in any_stage()) {
        let mut stage = from;
        let result = stage.advance_to(to);
        if to >= from {
            prop_assert!(result.is_ok());
            prop_assert_eq!(stage, to);
        } else {
            prop_assert!(result.is_err());
            prop_assert_eq!(stage, from, "a refused advance must leave the stage alone");
        }
    }

    #[test]
    fn no_sequence_of_advances_can_regress(steps in prop::collection::vec(any_stage(), 0..20)) {
        let mut stage = LifeStage::Egg;
        let mut highest = LifeStage::Egg;
        for target in steps {
            let _ = stage.advance_to(target);
            prop_assert!(stage >= highest);
            highest = stage;
        }
    }
}

// ---------------------------------------------------------------------------
// Invariant 3: integer arithmetic saturates and truncates toward zero (ADR 0006).
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn milli_arithmetic_never_wraps(a in any_milli(), b in any_milli()) {
        // Wrapping would flip a sign; saturating cannot.
        let sum = a.saturating_add(b);
        let expected = (a.0 as i64) + (b.0 as i64);
        prop_assert_eq!(sum.0 as i64, expected.clamp(i32::MIN as i64, i32::MAX as i64));
    }

    #[test]
    fn scaling_truncates_toward_zero(value in any_milli(), num in -1_000i32..1_000) {
        let scaled = value.scale(num, 1_000);
        let exact = (value.0 as i64 * num as i64) / 1_000;
        prop_assert_eq!(scaled.0 as i64, exact.clamp(i32::MIN as i64, i32::MAX as i64));
        // Truncation toward zero can never change the sign of the product.
        if scaled.0 != 0 {
            prop_assert_eq!(scaled.0.signum() as i64, exact.signum());
        }
    }

    #[test]
    fn applying_the_identity_rate_changes_nothing(value in any_milli()) {
        prop_assert_eq!(Fixed::ONE.apply(value), value);
    }

    #[test]
    fn applying_zero_yields_zero(value in any_milli()) {
        prop_assert_eq!(Fixed::ZERO.apply(value), Milli::ZERO);
    }

    #[test]
    fn fixed_round_trips_through_its_decimal_form(raw in any::<i64>()) {
        // The decimal form is exact because 2^-16 terminates in base 10.
        let value = Fixed::from_raw(raw);
        let text = value.to_string();
        prop_assert_eq!(text.parse::<Fixed>().unwrap(), value);
    }

    #[test]
    fn parsing_never_panics_on_arbitrary_text(text in ".*") {
        // Content packs are hand-edited and will be moddable; the parser must
        // reject, never crash.
        let _ = text.parse::<Fixed>();
    }
}

// ---------------------------------------------------------------------------
// Invariant 4: the clock never loses or invents time.
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn the_clock_equals_floor_elapsed_over_sixty_however_it_is_polled(
        mut polls in prop::collection::vec(0u64..1_000_000u64, 1..60),
    ) {
        // This is the offline/live equivalence property in miniature: how often
        // the game is opened must not change how much game time has passed.
        polls.sort_unstable();
        let mut clock = GameClock::starting_at(UnixSeconds(0));
        for wall in &polls {
            let owed = clock.ticks_owed(UnixSeconds(*wall as i64));
            clock.advance(owed);
        }
        let final_wall = *polls.last().expect("non-empty");
        prop_assert_eq!(clock.current(), Tick(final_wall / SECONDS_PER_TICK));
    }

    #[test]
    fn a_backwards_clock_never_advances_time(
        start in 0i64..1_000_000,
        backwards in 0i64..1_000_000,
    ) {
        let mut clock = GameClock::starting_at(UnixSeconds(start));
        let owed = clock.ticks_owed(UnixSeconds(start - backwards));
        prop_assert_eq!(owed, GameDuration::ZERO);
        clock.advance(owed);
        prop_assert_eq!(clock.current(), Tick::ZERO);
    }

    #[test]
    fn durations_between_ticks_are_never_negative(a in any::<u64>(), b in any::<u64>()) {
        let span = Tick(a).since(Tick(b));
        if a >= b {
            prop_assert_eq!(span.as_ticks(), a - b);
        } else {
            prop_assert_eq!(span, GameDuration::ZERO);
        }
    }
}

// ---------------------------------------------------------------------------
// Invariant 5: randomness is bounded, reproducible and stream-isolated.
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn draws_stay_within_their_bounds(
        seed in any::<u64>(),
        tick in any::<u64>(),
        entity in any::<u64>(),
        bound in 1u64..10_000,
    ) {
        let mut rng = rng_for(WorldSeed(seed), RngStream::LootRoll, Tick(tick), entity);
        for _ in 0..16 {
            prop_assert!(rng.below(bound) < bound);
        }
    }

    #[test]
    fn ranges_include_both_endpoints_and_nothing_else(
        seed in any::<u64>(),
        low in any::<i32>(),
        span in 0i32..1_000,
    ) {
        let high = low.saturating_add(span);
        let mut rng = rng_for(WorldSeed(seed), RngStream::CombatAi, Tick(0), 0);
        for _ in 0..16 {
            let value = rng.range_inclusive(low, high);
            prop_assert!(value >= low && value <= high);
        }
    }

    #[test]
    fn the_same_position_always_produces_the_same_draws(
        seed in any::<u64>(),
        tick in any::<u64>(),
        entity in any::<u64>(),
    ) {
        let draw = |()| {
            let mut rng = rng_for(WorldSeed(seed), RngStream::Mutation, Tick(tick), entity);
            (0..8).map(|_| rng.next_u64()).collect::<Vec<_>>()
        };
        prop_assert_eq!(draw(()), draw(()));
    }

    #[test]
    fn shuffling_is_a_permutation(seed in any::<u64>(), length in 0usize..64) {
        let mut items: Vec<usize> = (0..length).collect();
        let mut rng = rng_for(WorldSeed(seed), RngStream::BehaviorChoice, Tick(0), 0);
        rng.shuffle(&mut items);
        items.sort_unstable();
        prop_assert_eq!(items, (0..length).collect::<Vec<_>>());
    }
}

// ---------------------------------------------------------------------------
// Invariant 7: stats, personality and the bounded collections hold their shape.
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn stats_stay_within_their_band(points in any::<u16>()) {
        let value = StatValue::new(points);
        prop_assert!(value >= StatValue::MIN, "a zero stat breaks combat maths");
        prop_assert!(value <= StatValue::MAX);
    }

    #[test]
    fn scaling_a_stat_can_never_leave_the_band(points in any::<u16>(), raw in any::<i64>()) {
        let scaled = StatValue::new(points).scaled_by(Fixed::from_raw(raw));
        prop_assert!(scaled >= StatValue::MIN && scaled <= StatValue::MAX);
    }

    #[test]
    fn a_stats_total_matches_the_sum_of_its_parts(points in any::<u16>()) {
        let stats = Stats::uniform(points);
        let summed: u32 = stats.iter().map(|(_, value)| u32::from(value.get())).sum();
        prop_assert_eq!(stats.total(), summed);
    }

    #[test]
    fn personality_blending_is_commutative(values in prop::collection::vec(0i32..=100, 10)) {
        // Parent order must never leak into a child's temperament.
        let make = |slice: &[i32]| {
            let trait_at = |percent: i32| {
                domain::pet::TraitValue::new(Milli(percent * 1_000))
            };
            Personality::new(
                trait_at(slice[0]),
                trait_at(slice[1]),
                trait_at(slice[2]),
                trait_at(slice[3]),
                trait_at(slice[4]),
            )
        };
        let first = make(&values[..5]);
        let second = make(&values[5..]);
        prop_assert_eq!(first.blended_with(&second), second.blended_with(&first));
    }

    #[test]
    fn the_genetic_schema_check_is_exact(
        present in prop::collection::vec("[a-e]{3}", 0..6),
        expected in prop::collection::vec("[a-e]{3}", 0..6),
    ) {
        let profile = GeneticProfile::from_loci(present.iter().map(|name| {
            (
                GeneId::new(name).expect("generated ids are valid"),
                GenePair::homozygous(Allele::standard(1)),
            )
        }));
        let schema: Vec<GeneId> =
            expected.iter().map(|name| GeneId::new(name).expect("valid")).collect();

        let present_set: std::collections::BTreeSet<&String> = present.iter().collect();
        let expected_set: std::collections::BTreeSet<&String> = expected.iter().collect();

        prop_assert_eq!(
            profile.check_against_schema(&schema).is_ok(),
            present_set == expected_set,
            "the check must accept exactly the profiles whose loci match the schema"
        );
    }

    #[test]
    fn relationships_never_exceed_their_capacity(count in 0usize..200) {
        let mut relationships = Relationships::new();
        for ordinal in 0..count as u64 {
            relationships.adjust(PetId::derive(WorldSeed(1), Tick::ZERO, ordinal), Milli(1_000));
        }
        prop_assert!(relationships.count() <= Relationships::CAPACITY);
    }

    #[test]
    fn a_wallet_can_never_go_negative(
        start in 0i64..1_000_000,
        moves in prop::collection::vec(-2_000i64..2_000, 0..50),
    ) {
        let mut wallet = Wallet::with_balance(Currency(start));
        for amount in moves {
            // Errors are expected and ignored; the invariant is about the state
            // left behind, not about which calls succeeded.
            if amount >= 0 {
                let _ = wallet.deposit(Currency(amount));
            } else {
                let _ = wallet.withdraw(Currency(-amount));
            }
            prop_assert!(wallet.balance() >= Currency::ZERO);
        }
    }

    #[test]
    fn inventory_quantities_are_conserved(
        adds in prop::collection::vec(1u32..1_000, 0..30),
        removes in prop::collection::vec(1u32..1_000, 0..30),
    ) {
        let mut inventory = Inventory::default();
        let item = ItemId::new("berry").expect("valid item id");

        let mut expected = 0u32;
        for amount in adds {
            inventory.add(item.clone(), amount).expect("a single kind always fits");
            expected = expected.saturating_add(amount);
        }
        for amount in removes {
            if inventory.remove(&item, amount).is_ok() {
                expected -= amount;
            }
        }
        prop_assert_eq!(inventory.count(&item), expected);
    }
}

// ---------------------------------------------------------------------------
// Invariant 6: everything serialized survives a round trip unchanged.
//
// This is the precondition for save/load equivalence. If a type cannot make it
// through serde intact, no amount of care in `persistence` will save it.
// ---------------------------------------------------------------------------

fn assert_round_trip<T>(value: &T)
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + core::fmt::Debug,
{
    let encoded = serde_json::to_string(value).expect("serialize");
    let decoded: T = serde_json::from_str(&encoded).expect("deserialize");
    assert_eq!(*value, decoded, "round trip changed the value");
}

proptest! {
    #[test]
    fn scalar_types_round_trip(raw in any::<i32>(), wide in any::<i64>(), ticks in any::<u64>()) {
        assert_round_trip(&Milli(raw));
        assert_round_trip(&Fixed::from_raw(wide));
        assert_round_trip(&NeedValue::new(Milli(raw)));
        assert_round_trip(&Tick(ticks));
        assert_round_trip(&GameDuration::from_ticks(ticks));
        assert_round_trip(&UnixSeconds(wide));
    }

    #[test]
    fn aggregates_round_trip(needs in any_needs(), stage in any_stage()) {
        assert_round_trip(&needs);
        assert_round_trip(&stage);
    }

    #[test]
    fn identifiers_round_trip(seed in any::<u64>(), tick in any::<u64>(), ordinal in any::<u64>()) {
        assert_round_trip(&PetId::derive(WorldSeed(seed), Tick(tick), ordinal));
    }

    #[test]
    fn the_whole_pet_aggregate_round_trips(ordinal in any::<u64>(), percent in 0i32..=100) {
        // The precondition for save/load equivalence at the aggregate level: if
        // the root type cannot survive serde intact, nothing in `persistence`
        // can rescue it.
        let mut pet = Pet::hatch(
            PetId::derive(WorldSeed(1), Tick::ZERO, ordinal),
            SpeciesId::new("emberling").expect("valid species"),
            GeneticProfile::from_loci([(
                GeneId::new("coat_colour").expect("valid gene"),
                GenePair::new(Allele::dominant(1), Allele::recessive(2)),
            )]),
            Personality::neutral(),
            Parentage::Founder,
            Ancestry::founder(),
            Tick(42),
        );
        pet.set_need(NeedKind::Hunger, NeedValue::new(Milli(percent * 1_000)), Tick(43));
        pet.memories.record(MemoryKind::Fed, Tick(43), Milli(50_000));
        pet.habits.reinforce(Activity::Playing, Milli(1_000));
        pet.relationships.adjust(PetId::derive(WorldSeed(1), Tick::ZERO, ordinal ^ 1), Milli(500));
        pet.refresh_emotions();

        assert_round_trip(&pet);
    }

    #[test]
    fn events_round_trip(
        seed in any::<u64>(),
        ordinal in any::<u64>(),
        from in any_need(),
        to in any_need(),
    ) {
        let pet = PetId::derive(WorldSeed(seed), Tick(0), ordinal);
        let species = SpeciesId::new("emberling").expect("valid id");

        assert_round_trip(&DomainEvent::Pet(PetEvent::NeedChanged {
            pet,
            need: NeedKind::Hunger,
            from,
            to,
        }));
        assert_round_trip(&DomainEvent::Pet(PetEvent::Hatched {
            pet,
            species,
            at: Tick(7),
        }));
        assert_round_trip(&DomainEvent::Pet(PetEvent::Died {
            pet,
            cause: DeathCause::Neglect { need: NeedKind::Energy },
            at: Tick(9),
        }));
    }
}
