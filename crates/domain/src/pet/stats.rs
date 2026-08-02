//! Combat and growth statistics.

use core::fmt;

use serde::{Deserialize, Serialize};

use crate::units::Fixed;

/// Which statistic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum StatKind {
    /// Maximum health.
    Vitality,
    /// Physical damage dealt.
    Strength,
    /// Turn order and evasion.
    Agility,
    /// Accuracy and critical chance.
    Focus,
    /// Damage taken and status resistance.
    Resilience,
}

impl StatKind {
    /// Fixed order — see the note on [`super::NeedKind::ALL`]; anything that
    /// scans stats must do so identically on every machine.
    pub const ALL: [StatKind; 5] = [
        StatKind::Vitality,
        StatKind::Strength,
        StatKind::Agility,
        StatKind::Focus,
        StatKind::Resilience,
    ];

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            StatKind::Vitality => "vitality",
            StatKind::Strength => "strength",
            StatKind::Agility => "agility",
            StatKind::Focus => "focus",
            StatKind::Resilience => "resilience",
        }
    }
}

impl fmt::Display for StatKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A statistic in whole points.
///
/// Points rather than [`crate::units::Milli`]: combat multiplies stats together
/// constantly, and keeping the operands small keeps every intermediate far from
/// an overflow. Needs are a percentage of a bar; a stat is a count.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct StatValue(u16);

impl StatValue {
    pub const MIN: Self = StatValue(1);
    pub const MAX: Self = StatValue(9_999);

    /// Clamps into `1..=9_999`.
    ///
    /// The floor is one, not zero: a zero stat turns damage formulas into
    /// divide-by-zero or guarantees-a-miss edge cases that no amount of content
    /// balancing can rescue.
    #[must_use]
    pub const fn new(points: u16) -> Self {
        if points < Self::MIN.0 {
            Self::MIN
        } else if points > Self::MAX.0 {
            Self::MAX
        } else {
            StatValue(points)
        }
    }

    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }

    #[must_use]
    pub const fn raised_by(self, points: u16) -> Self {
        Self::new(self.0.saturating_add(points))
    }

    #[must_use]
    pub const fn lowered_by(self, points: u16) -> Self {
        Self::new(self.0.saturating_sub(points))
    }

    /// Applies a multiplier, truncating toward zero and re-clamping.
    #[must_use]
    pub const fn scaled_by(self, factor: Fixed) -> Self {
        let scaled = factor.apply(crate::units::Milli(self.0 as i32));
        Self::new(if scaled.0 < 0 {
            0
        } else if scaled.0 > u16::MAX as i32 {
            u16::MAX
        } else {
            scaled.0 as u16
        })
    }
}

/// A pet's five statistics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stats {
    pub vitality: StatValue,
    pub strength: StatValue,
    pub agility: StatValue,
    pub focus: StatValue,
    pub resilience: StatValue,
}

impl Stats {
    /// Every stat at the same value. The starting point before genetics and
    /// progression modify it.
    #[must_use]
    pub const fn uniform(points: u16) -> Self {
        let value = StatValue::new(points);
        Stats {
            vitality: value,
            strength: value,
            agility: value,
            focus: value,
            resilience: value,
        }
    }

    #[must_use]
    pub const fn get(&self, kind: StatKind) -> StatValue {
        match kind {
            StatKind::Vitality => self.vitality,
            StatKind::Strength => self.strength,
            StatKind::Agility => self.agility,
            StatKind::Focus => self.focus,
            StatKind::Resilience => self.resilience,
        }
    }

    pub const fn set(&mut self, kind: StatKind, value: StatValue) {
        match kind {
            StatKind::Vitality => self.vitality = value,
            StatKind::Strength => self.strength = value,
            StatKind::Agility => self.agility = value,
            StatKind::Focus => self.focus = value,
            StatKind::Resilience => self.resilience = value,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (StatKind, StatValue)> + '_ {
        StatKind::ALL.into_iter().map(|kind| (kind, self.get(kind)))
    }

    /// The sum of all five, as a rough power rating.
    #[must_use]
    pub fn total(&self) -> u32 {
        self.iter().map(|(_, value)| u32::from(value.get())).sum()
    }

    /// The highest stat, ties broken toward the earlier entry in
    /// [`StatKind::ALL`].
    #[must_use]
    pub fn strongest(&self) -> (StatKind, StatValue) {
        let mut best = (StatKind::Vitality, self.vitality);
        for (kind, value) in self.iter() {
            if value > best.1 {
                best = (kind, value);
            }
        }
        best
    }
}

impl Default for Stats {
    fn default() -> Self {
        Self::uniform(10)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_clamp_into_the_valid_band() {
        assert_eq!(
            StatValue::new(0),
            StatValue::MIN,
            "a zero stat breaks combat maths"
        );
        assert_eq!(StatValue::new(u16::MAX), StatValue::MAX);
        assert_eq!(StatValue::new(50).get(), 50);
    }

    #[test]
    fn raising_and_lowering_stay_in_band() {
        assert_eq!(StatValue::MAX.raised_by(1_000), StatValue::MAX);
        assert_eq!(StatValue::MIN.lowered_by(1_000), StatValue::MIN);
        assert_eq!(StatValue::new(10).raised_by(5).get(), 15);
    }

    #[test]
    fn scaling_truncates_and_clamps() {
        let stat = StatValue::new(100);
        assert_eq!(stat.scaled_by(Fixed::ONE), stat);
        assert_eq!(stat.scaled_by(Fixed::from_ratio(1, 2)).get(), 50);
        assert_eq!(
            stat.scaled_by(Fixed::ZERO),
            StatValue::MIN,
            "scaling to zero floors at one"
        );
        assert_eq!(stat.scaled_by(Fixed::from_int(1_000)), StatValue::MAX);
    }

    #[test]
    fn scaling_by_a_negative_rate_floors_rather_than_wrapping() {
        assert_eq!(
            StatValue::new(100).scaled_by(Fixed::from_int(-2)),
            StatValue::MIN
        );
    }

    #[test]
    fn every_stat_is_reachable_by_kind() {
        let mut stats = Stats::uniform(10);
        for (index, kind) in StatKind::ALL.into_iter().enumerate() {
            stats.set(kind, StatValue::new(20 + index as u16));
        }
        for (index, kind) in StatKind::ALL.into_iter().enumerate() {
            assert_eq!(
                stats.get(kind).get(),
                20 + index as u16,
                "{kind} aliased another field"
            );
        }
    }

    #[test]
    fn iteration_order_is_fixed() {
        let kinds: Vec<_> = Stats::default().iter().map(|(kind, _)| kind).collect();
        assert_eq!(kinds, StatKind::ALL.to_vec());
    }

    #[test]
    fn total_sums_every_stat() {
        assert_eq!(Stats::uniform(10).total(), 50);
    }

    #[test]
    fn strongest_finds_the_peak_and_breaks_ties_early() {
        let mut stats = Stats::uniform(10);
        stats.set(StatKind::Agility, StatValue::new(99));
        assert_eq!(stats.strongest().0, StatKind::Agility);

        // All equal: the first declared kind wins, deterministically.
        assert_eq!(Stats::uniform(10).strongest().0, StatKind::Vitality);
    }
}
