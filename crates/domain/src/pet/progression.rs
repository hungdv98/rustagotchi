//! Levels, experience and age.

use serde::{Deserialize, Serialize};

use crate::time::{GameDuration, Tick};

/// A pet's level, `1..=99`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Level(u16);

impl Level {
    pub const MIN: Self = Level(1);
    pub const MAX: Self = Level(99);

    #[must_use]
    pub const fn new(value: u16) -> Self {
        if value < Self::MIN.0 {
            Self::MIN
        } else if value > Self::MAX.0 {
            Self::MAX
        } else {
            Level(value)
        }
    }

    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }

    #[must_use]
    pub const fn is_max(self) -> bool {
        self.0 >= Self::MAX.0
    }
}

impl Default for Level {
    fn default() -> Self {
        Self::MIN
    }
}

/// Accumulated experience.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Experience(pub u32);

/// Level, experience and date of birth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PetProgression {
    level: Level,
    experience: Experience,
    born_at: Tick,
}

impl PetProgression {
    #[must_use]
    pub const fn born_at(born_at: Tick) -> Self {
        PetProgression {
            level: Level::MIN,
            experience: Experience(0),
            born_at,
        }
    }

    #[must_use]
    pub const fn level(&self) -> Level {
        self.level
    }

    #[must_use]
    pub const fn experience(&self) -> Experience {
        self.experience
    }

    #[must_use]
    pub const fn birth_tick(&self) -> Tick {
        self.born_at
    }

    #[must_use]
    pub const fn age(&self, now: Tick) -> GameDuration {
        now.since(self.born_at)
    }

    /// Experience needed to go from `level` to the next.
    ///
    /// `50 * level * (level + 1)` — a quadratic curve evaluated in integers, so
    /// the threshold is identical on every platform and in every replay. At
    /// level 1 that is 100; at level 98, 485,100.
    #[must_use]
    pub const fn required_for_next(level: Level) -> u32 {
        if level.is_max() {
            return u32::MAX;
        }
        let value = level.get() as u32;
        50 * value * (value + 1)
    }

    /// Total experience to reach `level` from level 1.
    #[must_use]
    pub const fn total_required_for(level: Level) -> u32 {
        let mut total = 0u32;
        let mut current = 1u16;
        while current < level.get() {
            total = total.saturating_add(Self::required_for_next(Level::new(current)));
            current += 1;
        }
        total
    }

    /// Awards experience, returning how many levels were gained.
    ///
    /// Loops rather than jumping straight to the final level: a single grant can
    /// legitimately cross several thresholds, and the caller usually wants to
    /// emit one event per level.
    pub fn grant(&mut self, amount: Experience) -> u16 {
        if self.level.is_max() {
            return 0;
        }
        self.experience = Experience(self.experience.0.saturating_add(amount.0));

        let mut gained = 0u16;
        loop {
            if self.level.is_max() {
                // At the cap, surplus experience is discarded rather than
                // accumulating into a number nothing will ever spend.
                self.experience = Experience(0);
                break;
            }
            let required = Self::required_for_next(self.level);
            if self.experience.0 < required {
                break;
            }
            self.experience = Experience(self.experience.0 - required);
            self.level = Level::new(self.level.get() + 1);
            gained += 1;
        }
        gained
    }

    /// Progress toward the next level, in thousandths.
    #[must_use]
    pub fn progress_to_next(&self) -> crate::units::Milli {
        if self.level.is_max() {
            return crate::units::Milli::ONE;
        }
        let required = Self::required_for_next(self.level);
        crate::units::Milli(self.experience.0 as i32).ratio_of(crate::units::Milli(required as i32))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::Milli;

    #[test]
    fn levels_clamp_into_range() {
        assert_eq!(Level::new(0), Level::MIN);
        assert_eq!(Level::new(1_000), Level::MAX);
        assert!(Level::MAX.is_max());
        assert!(!Level::MIN.is_max());
    }

    #[test]
    fn a_new_pet_starts_at_level_one_with_no_experience() {
        let progression = PetProgression::born_at(Tick(100));
        assert_eq!(progression.level(), Level::MIN);
        assert_eq!(progression.experience(), Experience(0));
        assert_eq!(progression.birth_tick(), Tick(100));
    }

    #[test]
    fn age_measures_from_birth_and_never_goes_negative() {
        let progression = PetProgression::born_at(Tick(100));
        assert_eq!(progression.age(Tick(1_540)), GameDuration::from_days(1));
        assert_eq!(progression.age(Tick(50)), GameDuration::ZERO);
    }

    #[test]
    fn the_experience_curve_grows_with_level() {
        assert_eq!(PetProgression::required_for_next(Level::new(1)), 100);
        assert_eq!(PetProgression::required_for_next(Level::new(2)), 300);
        assert_eq!(PetProgression::required_for_next(Level::new(10)), 5_500);
        assert_eq!(PetProgression::required_for_next(Level::MAX), u32::MAX);

        let mut previous = 0;
        for level in 1..99u16 {
            let required = PetProgression::required_for_next(Level::new(level));
            assert!(
                required > previous,
                "the curve must be strictly increasing at {level}"
            );
            previous = required;
        }
    }

    #[test]
    fn granting_enough_experience_levels_up() {
        let mut progression = PetProgression::born_at(Tick::ZERO);
        assert_eq!(progression.grant(Experience(99)), 0);
        assert_eq!(progression.level(), Level::new(1));

        assert_eq!(progression.grant(Experience(1)), 1);
        assert_eq!(progression.level(), Level::new(2));
        assert_eq!(progression.experience(), Experience(0));
    }

    #[test]
    fn a_single_grant_can_cross_several_levels() {
        let mut progression = PetProgression::born_at(Tick::ZERO);
        // 100 for level 2, 300 for level 3, 600 for level 4.
        let gained = progression.grant(Experience(1_000));
        assert_eq!(gained, 3);
        assert_eq!(progression.level(), Level::new(4));
        assert_eq!(progression.experience(), Experience(0));
    }

    #[test]
    fn leftover_experience_carries_toward_the_next_level() {
        let mut progression = PetProgression::born_at(Tick::ZERO);
        progression.grant(Experience(150));
        assert_eq!(progression.level(), Level::new(2));
        assert_eq!(progression.experience(), Experience(50));
    }

    #[test]
    fn the_level_cap_holds_against_an_absurd_grant() {
        let mut progression = PetProgression::born_at(Tick::ZERO);
        progression.grant(Experience(u32::MAX));
        assert_eq!(progression.level(), Level::MAX);

        // And a further grant is a no-op rather than an overflow.
        assert_eq!(progression.grant(Experience(u32::MAX)), 0);
        assert_eq!(progression.level(), Level::MAX);
    }

    #[test]
    fn experience_is_not_hoarded_at_the_cap() {
        let mut progression = PetProgression::born_at(Tick::ZERO);
        progression.grant(Experience(u32::MAX));
        assert_eq!(progression.experience(), Experience(0));
    }

    #[test]
    fn progress_reports_a_fraction_of_the_next_level() {
        let mut progression = PetProgression::born_at(Tick::ZERO);
        assert_eq!(progression.progress_to_next(), Milli::ZERO);
        progression.grant(Experience(50));
        assert_eq!(progression.progress_to_next(), Milli(500));
    }

    #[test]
    fn total_required_accumulates_the_curve() {
        assert_eq!(PetProgression::total_required_for(Level::new(1)), 0);
        assert_eq!(PetProgression::total_required_for(Level::new(2)), 100);
        // 100 + 300 + 600, matching the levels a 1_000 grant buys above.
        assert_eq!(PetProgression::total_required_for(Level::new(4)), 1_000);
    }

    #[test]
    fn total_required_agrees_with_granting_that_much() {
        // The two must not be able to drift apart: one drives the UI's progress
        // bar, the other drives actual level-ups.
        for level in 2..=20u16 {
            let target = Level::new(level);
            let mut progression = PetProgression::born_at(Tick::ZERO);
            progression.grant(Experience(PetProgression::total_required_for(target)));
            assert_eq!(
                progression.level(),
                target,
                "granting the stated total must reach {level}"
            );
            assert_eq!(progression.experience(), Experience(0));
        }
    }
}
