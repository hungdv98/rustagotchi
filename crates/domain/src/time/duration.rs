//! Spans of game time, counted in ticks.

use core::fmt;

use serde::{Deserialize, Serialize};

use super::game_time::{TICKS_PER_DAY, TICKS_PER_HOUR};

/// A span of game time, in whole ticks.
///
/// Unsigned: a duration is a magnitude. Use [`Tick::since`](super::Tick::since)
/// to measure between two instants, which saturates at zero rather than
/// producing a negative span.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct GameDuration(u64);

impl GameDuration {
    pub const ZERO: Self = GameDuration(0);
    pub const TICK: Self = GameDuration(1);

    #[must_use]
    pub const fn from_ticks(ticks: u64) -> Self {
        GameDuration(ticks)
    }

    /// One game-minute is one tick.
    #[must_use]
    pub const fn from_minutes(minutes: u64) -> Self {
        GameDuration(minutes)
    }

    #[must_use]
    pub const fn from_hours(hours: u64) -> Self {
        GameDuration(hours.saturating_mul(TICKS_PER_HOUR))
    }

    #[must_use]
    pub const fn from_days(days: u64) -> Self {
        GameDuration(days.saturating_mul(TICKS_PER_DAY))
    }

    #[must_use]
    pub const fn as_ticks(self) -> u64 {
        self.0
    }

    #[must_use]
    pub const fn whole_hours(self) -> u64 {
        self.0 / TICKS_PER_HOUR
    }

    #[must_use]
    pub const fn whole_days(self) -> u64 {
        self.0 / TICKS_PER_DAY
    }

    #[must_use]
    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    #[must_use]
    pub const fn saturating_add(self, rhs: Self) -> Self {
        GameDuration(self.0.saturating_add(rhs.0))
    }

    #[must_use]
    pub const fn saturating_sub(self, rhs: Self) -> Self {
        GameDuration(self.0.saturating_sub(rhs.0))
    }

    #[must_use]
    pub const fn saturating_mul(self, factor: u64) -> Self {
        GameDuration(self.0.saturating_mul(factor))
    }

    /// Clamps to at most `limit`, reporting whether anything was cut.
    ///
    /// Used by offline catch-up, which caps total elapsed time and records that
    /// it did so rather than silently simulating a different amount of time than
    /// actually passed.
    #[must_use]
    pub const fn clamped_to(self, limit: Self) -> (Self, bool) {
        if self.0 > limit.0 {
            (limit, true)
        } else {
            (self, false)
        }
    }
}

impl fmt::Display for GameDuration {
    /// Renders the largest sensible unit: `3d 4h 5m`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let days = self.0 / TICKS_PER_DAY;
        let hours = (self.0 % TICKS_PER_DAY) / TICKS_PER_HOUR;
        let minutes = self.0 % TICKS_PER_HOUR;
        match (days, hours, minutes) {
            (0, 0, m) => write!(f, "{m}m"),
            (0, h, m) => write!(f, "{h}h {m}m"),
            (d, h, m) => write!(f, "{d}d {h}h {m}m"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_constructors_agree() {
        assert_eq!(GameDuration::from_minutes(60), GameDuration::from_hours(1));
        assert_eq!(GameDuration::from_hours(24), GameDuration::from_days(1));
        assert_eq!(GameDuration::from_days(1).as_ticks(), 1_440);
        assert_eq!(GameDuration::from_days(365).as_ticks(), 525_600);
    }

    #[test]
    fn whole_unit_accessors_truncate() {
        let span = GameDuration::from_ticks(1_500);
        assert_eq!(span.whole_days(), 1);
        assert_eq!(span.whole_hours(), 25);
    }

    #[test]
    fn constructors_saturate_rather_than_overflowing() {
        assert_eq!(GameDuration::from_days(u64::MAX).as_ticks(), u64::MAX);
        assert_eq!(GameDuration::from_hours(u64::MAX).as_ticks(), u64::MAX);
    }

    #[test]
    fn subtraction_floors_at_zero() {
        let one_hour = GameDuration::from_hours(1);
        assert_eq!(
            one_hour.saturating_sub(GameDuration::from_days(1)),
            GameDuration::ZERO
        );
    }

    #[test]
    fn clamping_reports_whether_it_cut() {
        let limit = GameDuration::from_days(730);
        let (kept, clamped) = GameDuration::from_days(1).clamped_to(limit);
        assert_eq!(kept, GameDuration::from_days(1));
        assert!(!clamped);

        let (cut, clamped) = GameDuration::from_days(1_000).clamped_to(limit);
        assert_eq!(cut, limit);
        assert!(clamped);
    }

    #[test]
    fn display_picks_the_largest_useful_unit() {
        assert_eq!(GameDuration::from_minutes(5).to_string(), "5m");
        assert_eq!(GameDuration::from_minutes(65).to_string(), "1h 5m");
        assert_eq!(GameDuration::from_ticks(1_500).to_string(), "1d 1h 0m");
    }
}
