//! A wall clock a test can drive.
//!
//! Every time-dependent behaviour in the game is reachable without waiting,
//! because the core reads time through a port rather than calling the OS. This
//! is the fake on the other side of that port.

use core::cell::Cell;

use domain::time::UnixSeconds;

/// A clock that only moves when a test moves it.
///
/// Interior mutability so it can be shared behind the `&self` signature that
/// `application::ports::Clock` will use in Phase 4, without every test having to
/// juggle a `RefCell`.
#[derive(Debug)]
pub struct FakeClock {
    now: Cell<UnixSeconds>,
}

impl FakeClock {
    #[must_use]
    pub fn starting_at(now: UnixSeconds) -> Self {
        FakeClock {
            now: Cell::new(now),
        }
    }

    /// A clock at the Unix epoch. Fine for tests that only care about spans.
    #[must_use]
    pub fn at_epoch() -> Self {
        Self::starting_at(UnixSeconds::EPOCH)
    }

    #[must_use]
    pub fn now_unix(&self) -> UnixSeconds {
        self.now.get()
    }

    /// Moves the clock forward.
    pub fn advance_seconds(&self, seconds: i64) {
        self.now.set(self.now.get().saturating_add_secs(seconds));
    }

    /// Moves the clock forward by whole game-minutes.
    pub fn advance_ticks(&self, ticks: u64) {
        let seconds = ticks.saturating_mul(domain::time::SECONDS_PER_TICK);
        self.advance_seconds(i64::try_from(seconds).unwrap_or(i64::MAX));
    }

    /// Moves the clock *backwards*.
    ///
    /// Deliberately available: a machine whose clock jumps backwards after a
    /// timezone change or an NTP correction is real input, and offline
    /// progression has to handle it without simulating negative time.
    pub fn rewind_seconds(&self, seconds: i64) {
        self.advance_seconds(-seconds);
    }
}

impl Default for FakeClock {
    fn default() -> Self {
        Self::at_epoch()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fake_clock_only_moves_when_told() {
        let clock = FakeClock::starting_at(UnixSeconds(1_000));
        assert_eq!(clock.now_unix(), UnixSeconds(1_000));
        assert_eq!(clock.now_unix(), UnixSeconds(1_000));
    }

    #[test]
    fn advancing_by_ticks_matches_advancing_by_seconds() {
        let by_ticks = FakeClock::at_epoch();
        by_ticks.advance_ticks(90);

        let by_seconds = FakeClock::at_epoch();
        by_seconds.advance_seconds(90 * 60);

        assert_eq!(by_ticks.now_unix(), by_seconds.now_unix());
    }

    #[test]
    fn the_clock_can_be_rewound() {
        let clock = FakeClock::starting_at(UnixSeconds(1_000));
        clock.rewind_seconds(400);
        assert_eq!(clock.now_unix(), UnixSeconds(600));
    }
}
