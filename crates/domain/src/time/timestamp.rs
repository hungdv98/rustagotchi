//! Wall-clock instants.
//!
//! This crate cannot read a clock. [`UnixSeconds`] is a plain value produced by
//! `application::ports::Clock` and handed in, which is what lets every
//! time-dependent test run against a fake clock with no cfg-gating.

use serde::{Deserialize, Serialize};

/// Seconds since the Unix epoch, UTC.
///
/// Signed, so instants before 1970 are representable rather than wrapping. That
/// matters more than it sounds: a machine with a badly wrong clock is a real
/// source of hostile input to offline progression.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct UnixSeconds(pub i64);

impl UnixSeconds {
    pub const EPOCH: Self = UnixSeconds(0);

    #[must_use]
    pub const fn saturating_add_secs(self, seconds: i64) -> Self {
        UnixSeconds(self.0.saturating_add(seconds))
    }

    /// Seconds from `self` to `later`, or zero if `later` is not after `self`.
    ///
    /// Clamping at zero is deliberate: if the system clock moves backwards —
    /// a timezone change, an NTP correction, or a player rewinding it — offline
    /// progression must simulate nothing rather than a negative span.
    #[must_use]
    pub const fn seconds_until(self, later: Self) -> u64 {
        if later.0 <= self.0 {
            0
        } else {
            // Both are i64 and later > self, so the difference is positive and
            // fits in u64 even across the full signed range.
            (later.0 as i128 - self.0 as i128) as u64
        }
    }

    #[must_use]
    pub const fn is_before(self, other: Self) -> bool {
        self.0 < other.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measures_forward_spans() {
        let start = UnixSeconds(1_000);
        assert_eq!(start.seconds_until(UnixSeconds(1_060)), 60);
    }

    #[test]
    fn a_backwards_clock_yields_zero_not_a_huge_span() {
        let start = UnixSeconds(1_000);
        assert_eq!(start.seconds_until(UnixSeconds(500)), 0);
        assert_eq!(start.seconds_until(start), 0);
    }

    #[test]
    fn spans_across_the_signed_range_do_not_overflow() {
        assert_eq!(
            UnixSeconds(i64::MIN).seconds_until(UnixSeconds(i64::MAX)),
            u64::MAX
        );
    }

    #[test]
    fn addition_saturates() {
        assert_eq!(
            UnixSeconds(i64::MAX).saturating_add_secs(1),
            UnixSeconds(i64::MAX)
        );
    }
}
