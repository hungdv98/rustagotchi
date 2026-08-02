//! Ticks and the game clock.

use core::fmt;

use serde::{Deserialize, Serialize};

use super::duration::GameDuration;
use super::timestamp::UnixSeconds;

/// One tick is one game-minute.
pub const TICKS_PER_HOUR: u64 = 60;
pub const TICKS_PER_DAY: u64 = TICKS_PER_HOUR * 24;
pub const TICKS_PER_YEAR: u64 = TICKS_PER_DAY * 365;

/// One game-minute is one real minute.
pub const SECONDS_PER_TICK: u64 = 60;

/// The cap on a single offline catch-up, per ADR 0003.
///
/// Two years of ticks is roughly four seconds of simulation at the per-tick
/// budget. Beyond it, elapsed time is clamped and the fact is recorded, so the
/// UI can explain the discrepancy instead of the player silently losing a pet
/// they thought was safe.
pub const MAX_OFFLINE_CATCHUP_TICKS: u64 = TICKS_PER_DAY * 730;

/// A point in game time.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Tick(pub u64);

impl Tick {
    pub const ZERO: Self = Tick(0);

    #[must_use]
    pub const fn saturating_add(self, span: GameDuration) -> Self {
        Tick(self.0.saturating_add(span.as_ticks()))
    }

    #[must_use]
    pub const fn saturating_sub(self, span: GameDuration) -> Self {
        Tick(self.0.saturating_sub(span.as_ticks()))
    }

    /// The span from `earlier` to `self`, or zero if `earlier` is later.
    #[must_use]
    pub const fn since(self, earlier: Self) -> GameDuration {
        GameDuration::from_ticks(self.0.saturating_sub(earlier.0))
    }

    /// True on the first tick of every `period`.
    ///
    /// Systems that should not run every tick use this rather than keeping their
    /// own counter — a counter is state that has to be saved, migrated, and kept
    /// consistent between the live and offline paths.
    ///
    /// # Panics
    ///
    /// If `period` is zero.
    #[must_use]
    pub const fn is_boundary_of(self, period: GameDuration) -> bool {
        assert!(
            !period.is_zero(),
            "Tick::is_boundary_of requires a non-zero period"
        );
        self.0.is_multiple_of(period.as_ticks())
    }
}

impl fmt::Display for Tick {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "t{}", self.0)
    }
}

/// Tracks the current tick and its anchor in wall-clock time.
///
/// The anchor advances by exactly `SECONDS_PER_TICK` per tick simulated, never
/// by "however long it has been". Leftover seconds therefore carry forward
/// instead of being discarded, so a player who opens the game every 90 seconds
/// accumulates the same ticks as one who opens it every 30 minutes. Discarding
/// the remainder is a slow, invisible leak of game time and is exactly the kind
/// of drift that breaks offline/live equivalence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameClock {
    current: Tick,
    /// Wall-clock instant at which `current` began.
    anchored_at: UnixSeconds,
}

impl GameClock {
    #[must_use]
    pub const fn starting_at(wall_now: UnixSeconds) -> Self {
        GameClock {
            current: Tick::ZERO,
            anchored_at: wall_now,
        }
    }

    #[must_use]
    pub const fn current(&self) -> Tick {
        self.current
    }

    #[must_use]
    pub const fn anchored_at(&self) -> UnixSeconds {
        self.anchored_at
    }

    /// Whole ticks owed as of `wall_now`, before any cap is applied.
    ///
    /// Returns zero if the clock has moved backwards.
    #[must_use]
    pub const fn ticks_owed(&self, wall_now: UnixSeconds) -> GameDuration {
        GameDuration::from_ticks(self.anchored_at.seconds_until(wall_now) / SECONDS_PER_TICK)
    }

    /// Advances by `span`, moving the anchor by exactly the time consumed.
    pub const fn advance(&mut self, span: GameDuration) {
        self.advance_after_catch_up(span, span);
    }

    /// Advances game time by `simulated` while consuming all wall-clock
    /// `elapsed` ticks.
    ///
    /// Offline catch-up may deliberately cap the amount simulated. Consuming
    /// the full elapsed span prevents the discarded portion from being owed
    /// again on every subsequent launch.
    ///
    /// # Panics
    ///
    /// If `simulated` is greater than `elapsed`.
    pub const fn advance_after_catch_up(&mut self, simulated: GameDuration, elapsed: GameDuration) {
        assert!(
            simulated.as_ticks() <= elapsed.as_ticks(),
            "simulated time cannot exceed elapsed time"
        );
        self.current = self.current.saturating_add(simulated);
        let consumed = elapsed.as_ticks().saturating_mul(SECONDS_PER_TICK);
        // The anchor only ever moves forward, and by at most i64::MAX seconds.
        let consumed = if consumed > i64::MAX as u64 {
            i64::MAX
        } else {
            consumed as i64
        };
        self.anchored_at = self.anchored_at.saturating_add_secs(consumed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_constants_match_the_design() {
        assert_eq!(TICKS_PER_HOUR, 60);
        assert_eq!(TICKS_PER_DAY, 1_440);
        assert_eq!(TICKS_PER_YEAR, 525_600);
        assert_eq!(MAX_OFFLINE_CATCHUP_TICKS, 1_051_200);
    }

    #[test]
    fn since_never_goes_negative() {
        assert_eq!(Tick(10).since(Tick(4)), GameDuration::from_ticks(6));
        assert_eq!(Tick(4).since(Tick(10)), GameDuration::ZERO);
    }

    #[test]
    fn boundaries_fire_on_the_period() {
        let hourly = GameDuration::from_hours(1);
        assert!(Tick(0).is_boundary_of(hourly));
        assert!(Tick(60).is_boundary_of(hourly));
        assert!(!Tick(59).is_boundary_of(hourly));
    }

    #[test]
    fn a_fresh_clock_owes_nothing() {
        let clock = GameClock::starting_at(UnixSeconds(1_000));
        assert_eq!(clock.current(), Tick::ZERO);
        assert_eq!(clock.ticks_owed(UnixSeconds(1_000)), GameDuration::ZERO);
    }

    #[test]
    fn ticks_owed_truncates_to_whole_ticks() {
        let clock = GameClock::starting_at(UnixSeconds(0));
        assert_eq!(clock.ticks_owed(UnixSeconds(59)), GameDuration::ZERO);
        assert_eq!(
            clock.ticks_owed(UnixSeconds(60)),
            GameDuration::from_ticks(1)
        );
        assert_eq!(
            clock.ticks_owed(UnixSeconds(119)),
            GameDuration::from_ticks(1)
        );
    }

    #[test]
    fn a_backwards_wall_clock_owes_nothing() {
        let clock = GameClock::starting_at(UnixSeconds(10_000));
        assert_eq!(clock.ticks_owed(UnixSeconds(0)), GameDuration::ZERO);
    }

    #[test]
    fn partial_ticks_carry_forward_instead_of_being_lost() {
        // Polling every 90 seconds means each poll owes a whole tick plus 30
        // seconds of credit. Discarding that remainder would lose a tick every
        // second poll; carrying it forward keeps the total equal to
        // floor(elapsed / 60) at every point.
        let mut clock = GameClock::starting_at(UnixSeconds(0));

        for (wall, expected_owed, expected_total) in
            [(90, 1, 1), (180, 2, 3), (270, 1, 4), (360, 2, 6)]
        {
            let owed = clock.ticks_owed(UnixSeconds(wall));
            assert_eq!(
                owed,
                GameDuration::from_ticks(expected_owed),
                "owed at {wall}s"
            );
            clock.advance(owed);
            assert_eq!(clock.current(), Tick(expected_total), "total at {wall}s");
            assert_eq!(
                clock.current().0,
                wall as u64 / SECONDS_PER_TICK,
                "the clock must always equal floor(elapsed / 60)"
            );
        }
    }

    #[test]
    fn many_small_polls_equal_one_large_poll() {
        // The property the carry-forward rule exists to guarantee.
        let mut frequent = GameClock::starting_at(UnixSeconds(0));
        for second in (0..=3_600).step_by(10) {
            let owed = frequent.ticks_owed(UnixSeconds(second));
            frequent.advance(owed);
        }

        let mut rare = GameClock::starting_at(UnixSeconds(0));
        let owed = rare.ticks_owed(UnixSeconds(3_600));
        rare.advance(owed);

        assert_eq!(frequent.current(), rare.current());
        assert_eq!(frequent.current(), Tick(60));
    }

    #[test]
    fn advancing_moves_the_anchor_by_exactly_the_time_consumed() {
        let mut clock = GameClock::starting_at(UnixSeconds(1_000));
        clock.advance(GameDuration::from_hours(2));
        assert_eq!(clock.current(), Tick(120));
        assert_eq!(clock.anchored_at(), UnixSeconds(1_000 + 120 * 60));
    }

    #[test]
    fn capped_catch_up_consumes_discarded_wall_time_only_once() {
        let mut clock = GameClock::starting_at(UnixSeconds(0));
        clock.advance_after_catch_up(GameDuration::from_days(730), GameDuration::from_days(1_000));

        assert_eq!(clock.current(), Tick(TICKS_PER_DAY * 730));
        assert_eq!(
            clock.anchored_at(),
            UnixSeconds((TICKS_PER_DAY * 1_000 * SECONDS_PER_TICK) as i64)
        );
        assert_eq!(clock.ticks_owed(clock.anchored_at()), GameDuration::ZERO);
    }

    #[test]
    #[should_panic(expected = "non-zero period")]
    fn boundary_rejects_a_zero_period() {
        let _ = Tick(0).is_boundary_of(GameDuration::ZERO);
    }
}
