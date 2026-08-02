//! Personality: fixed at birth, never drifting.

use core::fmt;

use serde::{Deserialize, Serialize};

use crate::units::Milli;

/// A personality axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TraitKind {
    /// Willingness to fight and explore.
    Boldness,
    /// Desire for company.
    Sociability,
    /// Drive to investigate new things.
    Curiosity,
    /// Tolerance for routine and training.
    Discipline,
    /// Attachment to the player.
    Affection,
}

impl TraitKind {
    pub const ALL: [TraitKind; 5] = [
        TraitKind::Boldness,
        TraitKind::Sociability,
        TraitKind::Curiosity,
        TraitKind::Discipline,
        TraitKind::Affection,
    ];

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            TraitKind::Boldness => "boldness",
            TraitKind::Sociability => "sociability",
            TraitKind::Curiosity => "curiosity",
            TraitKind::Discipline => "discipline",
            TraitKind::Affection => "affection",
        }
    }
}

impl fmt::Display for TraitKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Where a pet sits on one axis, `0.000..=100.000`.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct TraitValue(Milli);

impl TraitValue {
    pub const MIN: Self = TraitValue(Milli::ZERO);
    pub const MAX: Self = TraitValue(Milli(100_000));
    /// The midpoint, `50.000`.
    pub const NEUTRAL: Self = TraitValue(Milli(50_000));

    #[must_use]
    pub const fn new(value: Milli) -> Self {
        TraitValue(value.clamp_to(Self::MIN.0, Self::MAX.0))
    }

    #[must_use]
    pub const fn get(self) -> Milli {
        self.0
    }

    #[must_use]
    pub const fn is_above(self, threshold: Self) -> bool {
        self.0.0 > threshold.0.0
    }
}

/// A pet's temperament.
///
/// **Immutable.** There is no setter, and every method returns a new value.
///
/// This is a design decision, not an oversight. The plan requires that
/// personality "never drifts under repeated updates"; the only way to guarantee
/// that is to make drift unrepresentable. Personality is decided once, at birth,
/// from genetics and parentage. The things that *should* change over a pet's
/// life — mood, habits, relationships, skills — are separate types that are free
/// to move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Personality {
    boldness: TraitValue,
    sociability: TraitValue,
    curiosity: TraitValue,
    discipline: TraitValue,
    affection: TraitValue,
}

impl Personality {
    #[must_use]
    pub const fn new(
        boldness: TraitValue,
        sociability: TraitValue,
        curiosity: TraitValue,
        discipline: TraitValue,
        affection: TraitValue,
    ) -> Self {
        Personality {
            boldness,
            sociability,
            curiosity,
            discipline,
            affection,
        }
    }

    /// Every axis at [`TraitValue::NEUTRAL`].
    #[must_use]
    pub const fn neutral() -> Self {
        let neutral = TraitValue::NEUTRAL;
        Personality::new(neutral, neutral, neutral, neutral, neutral)
    }

    #[must_use]
    pub const fn get(&self, kind: TraitKind) -> TraitValue {
        match kind {
            TraitKind::Boldness => self.boldness,
            TraitKind::Sociability => self.sociability,
            TraitKind::Curiosity => self.curiosity,
            TraitKind::Discipline => self.discipline,
            TraitKind::Affection => self.affection,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (TraitKind, TraitValue)> + '_ {
        TraitKind::ALL
            .into_iter()
            .map(|kind| (kind, self.get(kind)))
    }

    /// The midpoint of two personalities, used when a pet inherits from its
    /// parents.
    ///
    /// Commutative and idempotent, both tested below: blending A with B gives
    /// the same answer as blending B with A, and blending a personality with
    /// itself returns it unchanged. Without those two properties, parent order
    /// would leak into a pet's temperament.
    #[must_use]
    pub fn blended_with(&self, other: &Personality) -> Personality {
        let blend = |kind: TraitKind| {
            let sum = self.get(kind).get().saturating_add(other.get(kind).get());
            TraitValue::new(sum.scale(1, 2))
        };
        Personality::new(
            blend(TraitKind::Boldness),
            blend(TraitKind::Sociability),
            blend(TraitKind::Curiosity),
            blend(TraitKind::Discipline),
            blend(TraitKind::Affection),
        )
    }

    /// The axis furthest from neutral — the pet's defining quirk.
    ///
    /// Ties break toward the earlier entry in [`TraitKind::ALL`].
    #[must_use]
    pub fn dominant_trait(&self) -> TraitKind {
        let distance =
            |value: TraitValue| (value.get().0 - TraitValue::NEUTRAL.get().0).unsigned_abs();
        let mut best = (TraitKind::Boldness, distance(self.boldness));
        for (kind, value) in self.iter() {
            if distance(value) > best.1 {
                best = (kind, distance(value));
            }
        }
        best.0
    }
}

impl Default for Personality {
    fn default() -> Self {
        Self::neutral()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trait_at(percent: i32) -> TraitValue {
        TraitValue::new(Milli(percent * 1_000))
    }

    #[test]
    fn values_clamp_into_range() {
        assert_eq!(TraitValue::new(Milli(-1)), TraitValue::MIN);
        assert_eq!(TraitValue::new(Milli(999_999)), TraitValue::MAX);
    }

    #[test]
    fn every_axis_is_reachable() {
        let personality = Personality::new(
            trait_at(10),
            trait_at(20),
            trait_at(30),
            trait_at(40),
            trait_at(50),
        );
        let values: Vec<i32> = personality.iter().map(|(_, value)| value.get().0).collect();
        assert_eq!(values, [10_000, 20_000, 30_000, 40_000, 50_000]);
    }

    #[test]
    fn blending_is_commutative() {
        // Parent order must not leak into a child's temperament.
        let first = Personality::new(
            trait_at(10),
            trait_at(90),
            trait_at(0),
            trait_at(100),
            trait_at(25),
        );
        let second = Personality::new(
            trait_at(30),
            trait_at(10),
            trait_at(60),
            trait_at(0),
            trait_at(75),
        );
        assert_eq!(first.blended_with(&second), second.blended_with(&first));
    }

    #[test]
    fn blending_with_itself_changes_nothing() {
        let personality = Personality::new(
            trait_at(11),
            trait_at(23),
            trait_at(37),
            trait_at(49),
            trait_at(51),
        );
        assert_eq!(personality.blended_with(&personality), personality);
    }

    #[test]
    fn repeated_blending_does_not_drift() {
        // The invariant the plan calls for: no amount of re-application moves a
        // personality that is already the blend of itself.
        let personality = Personality::new(
            trait_at(20),
            trait_at(40),
            trait_at(60),
            trait_at(80),
            trait_at(100),
        );
        let mut blended = personality;
        for _ in 0..1_000 {
            blended = blended.blended_with(&personality);
        }
        assert_eq!(blended, personality);
    }

    #[test]
    fn blending_stays_between_the_two_inputs() {
        let low = Personality::new(
            trait_at(0),
            trait_at(0),
            trait_at(0),
            trait_at(0),
            trait_at(0),
        );
        let high = Personality::new(
            trait_at(100),
            trait_at(100),
            trait_at(100),
            trait_at(100),
            trait_at(100),
        );
        for (_, value) in low.blended_with(&high).iter() {
            assert_eq!(value, trait_at(50));
        }
    }

    #[test]
    fn the_dominant_trait_is_the_one_furthest_from_neutral() {
        let personality = Personality::new(
            trait_at(55),
            trait_at(50),
            trait_at(5),
            trait_at(50),
            trait_at(50),
        );
        assert_eq!(personality.dominant_trait(), TraitKind::Curiosity);
    }

    #[test]
    fn a_neutral_personality_breaks_its_tie_deterministically() {
        assert_eq!(Personality::neutral().dominant_trait(), TraitKind::Boldness);
    }
}
