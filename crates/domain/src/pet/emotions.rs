//! Emotional state and the mood it produces.

use core::fmt;

use serde::{Deserialize, Serialize};

use super::needs::PhysicalNeeds;
use crate::units::Milli;

/// A single feeling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Emotion {
    Joy,
    Sadness,
    Anger,
    Fear,
    Boredom,
}

impl Emotion {
    pub const ALL: [Emotion; 5] = [
        Emotion::Joy,
        Emotion::Sadness,
        Emotion::Anger,
        Emotion::Fear,
        Emotion::Boredom,
    ];

    /// Whether feeling this more is a good thing.
    #[must_use]
    pub const fn is_positive(self) -> bool {
        matches!(self, Emotion::Joy)
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Emotion::Joy => "joy",
            Emotion::Sadness => "sadness",
            Emotion::Anger => "anger",
            Emotion::Fear => "fear",
            Emotion::Boredom => "boredom",
        }
    }
}

impl fmt::Display for Emotion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// How strongly a feeling is felt, `0.000..=100.000`.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Intensity(Milli);

impl Intensity {
    pub const NONE: Self = Intensity(Milli::ZERO);
    pub const MAX: Self = Intensity(Milli(100_000));

    #[must_use]
    pub const fn new(value: Milli) -> Self {
        Intensity(value.clamp_to(Self::NONE.0, Self::MAX.0))
    }

    #[must_use]
    pub const fn get(self) -> Milli {
        self.0
    }

    #[must_use]
    pub const fn increased_by(self, amount: Milli) -> Self {
        Self::new(self.0.saturating_add(amount))
    }

    #[must_use]
    pub const fn decayed_by(self, amount: Milli) -> Self {
        Self::new(self.0.saturating_sub(amount))
    }
}

/// What a pet is feeling right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmotionalState {
    pub joy: Intensity,
    pub sadness: Intensity,
    pub anger: Intensity,
    pub fear: Intensity,
    pub boredom: Intensity,
}

impl EmotionalState {
    /// A settled pet: mild joy, nothing else.
    #[must_use]
    pub const fn content() -> Self {
        EmotionalState {
            joy: Intensity::new(Milli(60_000)),
            sadness: Intensity::NONE,
            anger: Intensity::NONE,
            fear: Intensity::NONE,
            boredom: Intensity::NONE,
        }
    }

    #[must_use]
    pub const fn get(&self, emotion: Emotion) -> Intensity {
        match emotion {
            Emotion::Joy => self.joy,
            Emotion::Sadness => self.sadness,
            Emotion::Anger => self.anger,
            Emotion::Fear => self.fear,
            Emotion::Boredom => self.boredom,
        }
    }

    pub const fn set(&mut self, emotion: Emotion, intensity: Intensity) {
        match emotion {
            Emotion::Joy => self.joy = intensity,
            Emotion::Sadness => self.sadness = intensity,
            Emotion::Anger => self.anger = intensity,
            Emotion::Fear => self.fear = intensity,
            Emotion::Boredom => self.boredom = intensity,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (Emotion, Intensity)> + '_ {
        Emotion::ALL
            .into_iter()
            .map(|emotion| (emotion, self.get(emotion)))
    }

    /// Net affect: joy minus the mean of the four negative feelings.
    ///
    /// The mean rather than the sum, so that adding a sixth negative emotion
    /// later does not silently make every pet miserable.
    #[must_use]
    pub fn affect(&self) -> Milli {
        let negatives: i32 = Emotion::ALL
            .into_iter()
            .filter(|emotion| !emotion.is_positive())
            .map(|emotion| self.get(emotion).get().0)
            .fold(0i32, i32::saturating_add);
        let negative_count = Emotion::ALL.iter().filter(|e| !e.is_positive()).count() as i32;
        self.joy
            .get()
            .saturating_sub(Milli(negatives).scale(1, negative_count))
    }

    /// The strongest feeling, ties broken toward the earlier entry in
    /// [`Emotion::ALL`].
    #[must_use]
    pub fn strongest(&self) -> (Emotion, Intensity) {
        let mut best = (Emotion::Joy, self.joy);
        for (emotion, intensity) in self.iter() {
            if intensity > best.1 {
                best = (emotion, intensity);
            }
        }
        best
    }

    /// The emotional state implied by a set of needs.
    ///
    /// Deliberately a pure function rather than an accumulating update: the
    /// alternative is a second piece of state that has to be kept in step with
    /// the needs across both the live and offline paths, which is precisely the
    /// kind of duplication that produces an equivalence failure.
    #[must_use]
    pub fn from_needs(needs: &PhysicalNeeds) -> Self {
        let (lowest_kind, lowest) = needs.lowest();
        let _ = lowest_kind;

        // Satisfaction maps straight onto joy; the shortfall drives distress.
        let shortfall = super::NeedValue::FULL.get().saturating_sub(lowest.get());

        EmotionalState {
            joy: Intensity::new(lowest.get()),
            sadness: Intensity::new(shortfall.scale(3, 4)),
            anger: Intensity::new(shortfall.scale(1, 4)),
            fear: Intensity::new(shortfall.scale(1, 5)),
            boredom: Intensity::new(super::NeedValue::FULL.get().saturating_sub(needs.fun.get())),
        }
    }
}

impl Default for EmotionalState {
    fn default() -> Self {
        Self::content()
    }
}

/// The banded summary shown to the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Mood {
    Miserable,
    Unhappy,
    Neutral,
    Content,
    Happy,
    Elated,
}

impl Mood {
    /// Ascending order.
    pub const ALL: [Mood; 6] = [
        Mood::Miserable,
        Mood::Unhappy,
        Mood::Neutral,
        Mood::Content,
        Mood::Happy,
        Mood::Elated,
    ];

    /// Bands over [`EmotionalState::affect`], which runs `-100.000..=100.000`.
    ///
    /// Integer thresholds, so the band a pet is in is exactly reproducible —
    /// a float comparison here would let two runs disagree about whether a pet
    /// looked happy.
    #[must_use]
    pub const fn from_affect(affect: Milli) -> Self {
        match affect.0 {
            value if value <= -50_000 => Mood::Miserable,
            value if value <= -15_000 => Mood::Unhappy,
            value if value < 15_000 => Mood::Neutral,
            value if value < 50_000 => Mood::Content,
            value if value < 85_000 => Mood::Happy,
            _ => Mood::Elated,
        }
    }

    #[must_use]
    pub fn of(state: &EmotionalState) -> Self {
        Self::from_affect(state.affect())
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Mood::Miserable => "miserable",
            Mood::Unhappy => "unhappy",
            Mood::Neutral => "neutral",
            Mood::Content => "content",
            Mood::Happy => "happy",
            Mood::Elated => "elated",
        }
    }
}

impl fmt::Display for Mood {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pet::{NeedKind, NeedValue};

    fn intensity(percent: i32) -> Intensity {
        Intensity::new(Milli(percent * 1_000))
    }

    #[test]
    fn intensities_clamp_into_range() {
        assert_eq!(Intensity::new(Milli(-1)), Intensity::NONE);
        assert_eq!(Intensity::new(Milli(999_999)), Intensity::MAX);
        assert_eq!(Intensity::MAX.increased_by(Milli(1_000)), Intensity::MAX);
        assert_eq!(Intensity::NONE.decayed_by(Milli(1_000)), Intensity::NONE);
    }

    #[test]
    fn every_emotion_is_reachable() {
        let mut state = EmotionalState::content();
        for (index, emotion) in Emotion::ALL.into_iter().enumerate() {
            state.set(emotion, intensity(index as i32 * 10));
        }
        for (index, emotion) in Emotion::ALL.into_iter().enumerate() {
            assert_eq!(
                state.get(emotion),
                intensity(index as i32 * 10),
                "{emotion} aliased"
            );
        }
    }

    #[test]
    fn affect_is_joy_minus_the_mean_of_the_negatives() {
        let mut state = EmotionalState {
            joy: intensity(100),
            sadness: intensity(20),
            anger: intensity(20),
            fear: intensity(20),
            boredom: intensity(20),
        };
        assert_eq!(state.affect(), Milli(80_000));

        state.joy = Intensity::NONE;
        assert_eq!(state.affect(), Milli(-20_000));
    }

    #[test]
    fn affect_spans_the_full_range() {
        let elated = EmotionalState {
            joy: Intensity::MAX,
            sadness: Intensity::NONE,
            anger: Intensity::NONE,
            fear: Intensity::NONE,
            boredom: Intensity::NONE,
        };
        let wretched = EmotionalState {
            joy: Intensity::NONE,
            sadness: Intensity::MAX,
            anger: Intensity::MAX,
            fear: Intensity::MAX,
            boredom: Intensity::MAX,
        };
        assert_eq!(elated.affect(), Milli(100_000));
        assert_eq!(wretched.affect(), Milli(-100_000));
    }

    #[test]
    fn moods_cover_the_whole_affect_range_in_order() {
        let moods: Vec<Mood> = [-100_000, -50_000, -20_000, 0, 30_000, 60_000, 100_000]
            .into_iter()
            .map(|affect| Mood::from_affect(Milli(affect)))
            .collect();
        assert_eq!(
            moods,
            [
                Mood::Miserable,
                Mood::Miserable,
                Mood::Unhappy,
                Mood::Neutral,
                Mood::Content,
                Mood::Happy,
                Mood::Elated
            ]
        );
    }

    #[test]
    fn mood_is_monotonic_in_affect() {
        let mut previous = Mood::Miserable;
        for affect in (-100_000..=100_000).step_by(1_000) {
            let mood = Mood::from_affect(Milli(affect));
            assert!(mood >= previous, "mood went backwards at affect {affect}");
            previous = mood;
        }
    }

    #[test]
    fn a_satisfied_pet_is_happy_and_a_neglected_one_is_not() {
        let satisfied = EmotionalState::from_needs(&PhysicalNeeds::full());
        assert!(Mood::of(&satisfied) >= Mood::Happy);

        let mut neglected = PhysicalNeeds::full();
        for kind in NeedKind::ALL {
            neglected.set(kind, NeedValue::EMPTY);
        }
        assert_eq!(
            Mood::of(&EmotionalState::from_needs(&neglected)),
            Mood::Miserable
        );
    }

    #[test]
    fn emotions_derived_from_needs_are_a_pure_function() {
        let needs = PhysicalNeeds::full();
        assert_eq!(
            EmotionalState::from_needs(&needs),
            EmotionalState::from_needs(&needs)
        );
    }

    #[test]
    fn the_strongest_emotion_breaks_ties_deterministically() {
        let mut state = EmotionalState::content();
        state.set(Emotion::Fear, Intensity::MAX);
        assert_eq!(state.strongest().0, Emotion::Fear);

        let flat = EmotionalState {
            joy: intensity(10),
            sadness: intensity(10),
            anger: intensity(10),
            fear: intensity(10),
            boredom: intensity(10),
        };
        assert_eq!(flat.strongest().0, Emotion::Joy);
    }
}
