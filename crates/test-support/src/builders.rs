//! Builders for domain values.
//!
//! A test should say what it is about. `PetBuilder::new().adult().starving()`
//! says "a starving adult"; twelve lines of struct literal says "a pet", and the
//! reader has to work out which field mattered.

use domain::genetics::{Ancestry, GeneticProfile, Parentage};
use domain::identifiers::{PetId, SpeciesId};
use domain::pet::{
    LifeStage, NeedKind, NeedValue, Personality, Pet, PhysicalNeeds, StatKind, StatValue, Stats,
};
use domain::rng::WorldSeed;
use domain::time::Tick;
use domain::units::Milli;

use crate::seeded_rng::TEST_SEED;

/// Builds a [`PhysicalNeeds`], starting fully satisfied.
#[derive(Debug, Clone)]
pub struct NeedsBuilder {
    needs: PhysicalNeeds,
}

impl NeedsBuilder {
    #[must_use]
    pub fn new() -> Self {
        NeedsBuilder {
            needs: PhysicalNeeds::full(),
        }
    }

    /// Sets one need to an explicit percentage, `0..=100`.
    #[must_use]
    pub fn with(mut self, kind: NeedKind, percent: i32) -> Self {
        self.needs
            .set(kind, NeedValue::new(Milli(percent.saturating_mul(1_000))));
        self
    }

    /// Hunger at zero. The classic neglect case.
    #[must_use]
    pub fn starving(self) -> Self {
        self.with(NeedKind::Hunger, 0)
    }

    #[must_use]
    pub fn exhausted(self) -> Self {
        self.with(NeedKind::Energy, 0)
    }

    #[must_use]
    pub fn filthy(self) -> Self {
        self.with(NeedKind::Hygiene, 0)
    }

    #[must_use]
    pub fn lonely(self) -> Self {
        self.with(NeedKind::Social, 5)
    }

    /// Every need at zero.
    #[must_use]
    pub fn neglected(mut self) -> Self {
        for kind in NeedKind::ALL {
            self.needs.set(kind, NeedValue::EMPTY);
        }
        self
    }

    #[must_use]
    pub fn build(self) -> PhysicalNeeds {
        self.needs
    }
}

impl Default for NeedsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builds a [`Pet`].
///
/// Defaults to a healthy, fully satisfied hatchling of species `emberling`,
/// born at [`Tick::ZERO`] in the [`TEST_SEED`] world.
#[derive(Debug, Clone)]
pub struct PetBuilder {
    ordinal: u64,
    species: SpeciesId,
    stage: LifeStage,
    needs: PhysicalNeeds,
    personality: Personality,
    genetics: GeneticProfile,
    stats: Stats,
    parentage: Parentage,
    ancestry: Ancestry,
    born_at: Tick,
}

impl PetBuilder {
    #[must_use]
    pub fn new() -> Self {
        PetBuilder {
            ordinal: 1,
            species: SpeciesId::new("emberling").expect("valid species id"),
            stage: LifeStage::Hatchling,
            needs: PhysicalNeeds::full(),
            personality: Personality::neutral(),
            genetics: GeneticProfile::new(),
            stats: Stats::default(),
            parentage: Parentage::Founder,
            ancestry: Ancestry::founder(),
            born_at: Tick::ZERO,
        }
    }

    /// Distinguishes this pet from others in the same test.
    #[must_use]
    pub fn ordinal(mut self, ordinal: u64) -> Self {
        self.ordinal = ordinal;
        self
    }

    /// # Panics
    ///
    /// If `species` is not a valid `snake_case` id — a test that names a species
    /// wrongly should fail loudly rather than quietly using a default.
    #[must_use]
    pub fn species(mut self, species: &str) -> Self {
        self.species = SpeciesId::new(species).expect("valid species id");
        self
    }

    #[must_use]
    pub fn stage(mut self, stage: LifeStage) -> Self {
        self.stage = stage;
        self
    }

    #[must_use]
    pub fn adult(self) -> Self {
        self.stage(LifeStage::Adult)
    }

    #[must_use]
    pub fn elder(self) -> Self {
        self.stage(LifeStage::Elder)
    }

    #[must_use]
    pub fn needs(mut self, needs: PhysicalNeeds) -> Self {
        self.needs = needs;
        self
    }

    #[must_use]
    pub fn starving(self) -> Self {
        let needs = NeedsBuilder::new().starving().build();
        self.needs(needs)
    }

    #[must_use]
    pub fn neglected(self) -> Self {
        let needs = NeedsBuilder::new().neglected().build();
        self.needs(needs)
    }

    #[must_use]
    pub fn personality(mut self, personality: Personality) -> Self {
        self.personality = personality;
        self
    }

    #[must_use]
    pub fn genetics(mut self, genetics: GeneticProfile) -> Self {
        self.genetics = genetics;
        self
    }

    /// Sets every stat to the same value.
    #[must_use]
    pub fn stats_at(mut self, points: u16) -> Self {
        self.stats = Stats::uniform(points);
        self
    }

    #[must_use]
    pub fn stat(mut self, kind: StatKind, points: u16) -> Self {
        self.stats.set(kind, StatValue::new(points));
        self
    }

    #[must_use]
    pub fn born_at(mut self, born_at: Tick) -> Self {
        self.born_at = born_at;
        self
    }

    #[must_use]
    pub fn bred_from(mut self, parents: [PetId; 2], ancestry: Ancestry) -> Self {
        self.parentage = Parentage::Bred { parents };
        self.ancestry = ancestry;
        self
    }

    #[must_use]
    pub fn build(self) -> Pet {
        let mut pet = Pet::hatch(
            PetId::derive(TEST_SEED, Tick::ZERO, self.ordinal),
            self.species,
            self.genetics,
            self.personality,
            self.parentage,
            self.ancestry,
            self.born_at,
        );
        pet.stage = self.stage;
        pet.needs = self.needs;
        pet.stats = self.stats;
        // Emotions are derived, so they must be recomputed after the needs are
        // overridden or the pet would be built already inconsistent.
        pet.refresh_emotions();
        pet
    }
}

impl Default for PetBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// A stable pet id for tests, distinguished by `ordinal`.
///
/// Derived from [`TEST_SEED`], so the same ordinal always yields the same id and
/// a failing assertion quotes an id you can look up.
#[must_use]
pub fn test_pet_id(ordinal: u64) -> PetId {
    PetId::derive(TEST_SEED, Tick::ZERO, ordinal)
}

/// A pet id in a world other than [`TEST_SEED`], for tests that need two worlds.
#[must_use]
pub fn pet_id_in_world(seed: WorldSeed, ordinal: u64) -> PetId {
    PetId::derive(seed, Tick::ZERO, ordinal)
}

/// The stage a test means when it says "an ordinary adult pet".
pub const DEFAULT_STAGE: LifeStage = LifeStage::Adult;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_build_is_fully_satisfied() {
        let needs = NeedsBuilder::new().build();
        assert!(needs.iter().all(|(_, value)| value.is_full()));
    }

    #[test]
    fn named_states_set_the_need_they_claim() {
        assert!(NeedsBuilder::new().starving().build().hunger.is_empty());
        assert!(NeedsBuilder::new().exhausted().build().energy.is_empty());
        assert!(NeedsBuilder::new().filthy().build().hygiene.is_empty());
    }

    #[test]
    fn named_states_leave_other_needs_alone() {
        let needs = NeedsBuilder::new().starving().build();
        assert!(needs.energy.is_full());
        assert!(needs.fun.is_full());
    }

    #[test]
    fn percentages_map_onto_the_scale() {
        let needs = NeedsBuilder::new().with(NeedKind::Fun, 50).build();
        assert_eq!(needs.fun.get(), Milli(50_000));
    }

    #[test]
    fn neglected_empties_everything() {
        let needs = NeedsBuilder::new().neglected().build();
        assert!(needs.iter().all(|(_, value)| value.is_empty()));
    }

    #[test]
    fn test_ids_are_stable_and_distinct() {
        assert_eq!(test_pet_id(1), test_pet_id(1));
        assert_ne!(test_pet_id(1), test_pet_id(2));
    }

    #[test]
    fn a_default_pet_is_a_healthy_hatchling() {
        let pet = PetBuilder::new().build();
        assert!(pet.is_alive());
        assert_eq!(pet.stage, LifeStage::Hatchling);
        assert!(pet.critical_needs().is_empty());
    }

    #[test]
    fn the_builder_applies_what_it_is_told() {
        let pet = PetBuilder::new()
            .ordinal(7)
            .species("stormling")
            .adult()
            .starving()
            .stats_at(42)
            .born_at(Tick(500))
            .build();

        assert_eq!(pet.id, test_pet_id(7));
        assert_eq!(pet.species.as_str(), "stormling");
        assert_eq!(pet.stage, LifeStage::Adult);
        assert!(pet.needs.hunger.is_empty());
        assert_eq!(pet.stats.strength.get(), 42);
        assert_eq!(pet.progression.birth_tick(), Tick(500));
    }

    #[test]
    fn a_built_pets_emotions_match_its_needs() {
        // The trap this guards: overriding needs without recomputing emotions
        // produces a starving pet that reports itself delighted.
        let pet = PetBuilder::new().neglected().build();
        assert_eq!(pet.mood(), domain::pet::Mood::Miserable);
    }

    #[test]
    fn distinct_ordinals_build_distinct_pets() {
        assert_ne!(
            PetBuilder::new().ordinal(1).build().id,
            PetBuilder::new().ordinal(2).build().id
        );
    }
}
