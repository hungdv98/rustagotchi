//! How a pet feels about other pets.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::identifiers::PetId;
use crate::units::Milli;

/// Feeling toward another pet, `-100.000..=100.000`.
///
/// Signed, unlike needs and stats: indifference is the middle of the range, not
/// the bottom, and hostility has to be representable for rivalries to mean
/// anything.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Affinity(Milli);

impl Affinity {
    pub const HOSTILE: Self = Affinity(Milli(-100_000));
    pub const INDIFFERENT: Self = Affinity(Milli::ZERO);
    pub const DEVOTED: Self = Affinity(Milli(100_000));

    /// At or above this counts as a friend.
    pub const FRIEND_THRESHOLD: Self = Affinity(Milli(40_000));
    /// At or below this counts as a rival.
    pub const RIVAL_THRESHOLD: Self = Affinity(Milli(-40_000));

    #[must_use]
    pub const fn new(value: Milli) -> Self {
        Affinity(value.clamp_to(Self::HOSTILE.0, Self::DEVOTED.0))
    }

    #[must_use]
    pub const fn get(self) -> Milli {
        self.0
    }

    #[must_use]
    pub const fn shifted_by(self, delta: Milli) -> Self {
        Self::new(self.0.saturating_add(delta))
    }

    #[must_use]
    pub const fn is_friendly(self) -> bool {
        self.0.0 >= Self::FRIEND_THRESHOLD.0.0
    }

    #[must_use]
    pub const fn is_hostile(self) -> bool {
        self.0.0 <= Self::RIVAL_THRESHOLD.0.0
    }
}

/// Every pet this one has an opinion about.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Relationships {
    affinities: BTreeMap<PetId, Affinity>,
}

impl Relationships {
    /// A pet cannot track opinions about more than this many others.
    ///
    /// Bounded for the same reason memories are: a long-lived save in a large
    /// colony would otherwise grow without limit.
    pub const CAPACITY: usize = 64;

    #[must_use]
    pub fn new() -> Self {
        Relationships::default()
    }

    /// Feeling toward `other`. Unknown pets are [`Affinity::INDIFFERENT`].
    #[must_use]
    pub fn toward(&self, other: PetId) -> Affinity {
        self.affinities
            .get(&other)
            .copied()
            .unwrap_or(Affinity::INDIFFERENT)
    }

    /// Adjusts feeling toward `other`, creating the entry if needed.
    ///
    /// Returns `false` if the relationship was new and there was no room. The
    /// caller can then decide whether to evict something; silently dropping the
    /// interaction would make a pet mysteriously indifferent to a friend.
    pub fn adjust(&mut self, other: PetId, delta: Milli) -> bool {
        match self.affinities.get(&other).copied() {
            Some(current) => {
                self.affinities.insert(other, current.shifted_by(delta));
                true
            }
            None if self.affinities.len() >= Self::CAPACITY => false,
            None => {
                self.affinities
                    .insert(other, Affinity::INDIFFERENT.shifted_by(delta));
                true
            }
        }
    }

    /// Removes every opinion about a pet that has left the world.
    pub fn forget(&mut self, other: PetId) -> Option<Affinity> {
        self.affinities.remove(&other)
    }

    /// Drops the relationship the pet cares least about, by absolute strength.
    ///
    /// Ties break toward the lower [`PetId`], so eviction is reproducible.
    pub fn forget_weakest(&mut self) -> Option<PetId> {
        let weakest = self
            .affinities
            .iter()
            .min_by_key(|(id, affinity)| (affinity.get().0.unsigned_abs(), **id))
            .map(|(id, _)| *id)?;
        self.affinities.remove(&weakest);
        Some(weakest)
    }

    pub fn friends(&self) -> impl Iterator<Item = (PetId, Affinity)> + '_ {
        self.affinities
            .iter()
            .filter(|(_, affinity)| affinity.is_friendly())
            .map(|(id, affinity)| (*id, *affinity))
    }

    pub fn rivals(&self) -> impl Iterator<Item = (PetId, Affinity)> + '_ {
        self.affinities
            .iter()
            .filter(|(_, affinity)| affinity.is_hostile())
            .map(|(id, affinity)| (*id, *affinity))
    }

    /// The pet it likes most, if it likes anyone at all.
    #[must_use]
    pub fn closest(&self) -> Option<(PetId, Affinity)> {
        let mut best: Option<(PetId, Affinity)> = None;
        for (id, affinity) in &self.affinities {
            match best {
                Some((_, current)) if *affinity <= current => {}
                _ => best = Some((*id, *affinity)),
            }
        }
        best
    }

    #[must_use]
    pub fn count(&self) -> usize {
        self.affinities.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.affinities.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::WorldSeed;
    use crate::time::Tick;

    fn pet(ordinal: u64) -> PetId {
        PetId::derive(WorldSeed(1), Tick::ZERO, ordinal)
    }

    #[test]
    fn unknown_pets_are_met_with_indifference() {
        let relationships = Relationships::new();
        assert_eq!(relationships.toward(pet(1)), Affinity::INDIFFERENT);
        assert!(relationships.is_empty());
    }

    #[test]
    fn affinity_clamps_at_both_ends() {
        assert_eq!(Affinity::new(Milli(-999_999)), Affinity::HOSTILE);
        assert_eq!(Affinity::new(Milli(999_999)), Affinity::DEVOTED);
        assert_eq!(
            Affinity::DEVOTED.shifted_by(Milli(i32::MAX)),
            Affinity::DEVOTED
        );
        assert_eq!(
            Affinity::HOSTILE.shifted_by(Milli(i32::MIN)),
            Affinity::HOSTILE
        );
    }

    #[test]
    fn adjustments_accumulate_in_both_directions() {
        let mut relationships = Relationships::new();
        assert!(relationships.adjust(pet(1), Milli(20_000)));
        assert!(relationships.adjust(pet(1), Milli(30_000)));
        assert_eq!(relationships.toward(pet(1)).get(), Milli(50_000));

        relationships.adjust(pet(1), Milli(-100_000));
        assert_eq!(relationships.toward(pet(1)).get(), Milli(-50_000));
    }

    #[test]
    fn friends_and_rivals_are_classified_by_threshold() {
        let mut relationships = Relationships::new();
        relationships.adjust(pet(1), Milli(50_000));
        relationships.adjust(pet(2), Milli(-50_000));
        relationships.adjust(pet(3), Milli(1_000));

        assert_eq!(relationships.friends().count(), 1);
        assert_eq!(relationships.rivals().count(), 1);
        assert!(!relationships.toward(pet(3)).is_friendly());
        assert!(!relationships.toward(pet(3)).is_hostile());
    }

    #[test]
    fn capacity_is_bounded_and_reported_rather_than_silent() {
        let mut relationships = Relationships::new();
        for ordinal in 0..Relationships::CAPACITY as u64 {
            assert!(relationships.adjust(pet(ordinal), Milli(1_000)));
        }
        assert!(
            !relationships.adjust(pet(9_999), Milli(1_000)),
            "a full relationship table must report the refusal"
        );

        // An existing relationship can still be updated at capacity.
        assert!(relationships.adjust(pet(0), Milli(1_000)));
    }

    #[test]
    fn forgetting_drops_the_least_strongly_felt() {
        let mut relationships = Relationships::new();
        relationships.adjust(pet(1), Milli(80_000));
        relationships.adjust(pet(2), Milli(-90_000));
        relationships.adjust(pet(3), Milli(500));

        assert_eq!(relationships.forget_weakest(), Some(pet(3)));
        assert_eq!(
            relationships.count(),
            2,
            "strong feelings survive, in either direction"
        );
    }

    #[test]
    fn forgetting_from_an_empty_table_is_a_no_op() {
        assert_eq!(Relationships::new().forget_weakest(), None);
    }

    #[test]
    fn the_closest_relationship_is_reported() {
        let mut relationships = Relationships::new();
        relationships.adjust(pet(1), Milli(10_000));
        relationships.adjust(pet(2), Milli(90_000));
        assert_eq!(relationships.closest().map(|(id, _)| id), Some(pet(2)));
    }
}
