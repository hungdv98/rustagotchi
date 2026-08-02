//! What a pet is currently doing.

use core::fmt;

use serde::{Deserialize, Serialize};

use crate::time::{GameDuration, Tick};

/// A thing a pet can be doing.
///
/// `Ord` because habits are keyed by activity in a `BTreeMap`, and that ordering
/// is what makes habit iteration deterministic.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub enum Activity {
    #[default]
    Idle,
    Eating,
    Sleeping,
    Playing,
    Exploring,
    Socialising,
    Bathing,
    Battling,
    /// Immobilised by illness. Not chosen; imposed.
    Recovering,
}

impl Activity {
    pub const ALL: [Activity; 9] = [
        Activity::Idle,
        Activity::Eating,
        Activity::Sleeping,
        Activity::Playing,
        Activity::Exploring,
        Activity::Socialising,
        Activity::Bathing,
        Activity::Battling,
        Activity::Recovering,
    ];

    /// Whether the pet chose this, as opposed to having it imposed.
    ///
    /// Only voluntary activities reinforce a habit — a pet laid up with an
    /// illness is not developing a taste for being ill.
    #[must_use]
    pub const fn is_voluntary(self) -> bool {
        !matches!(self, Activity::Recovering)
    }

    /// Whether the activity blocks a new one being started.
    #[must_use]
    pub const fn is_interruptible(self) -> bool {
        matches!(
            self,
            Activity::Idle | Activity::Playing | Activity::Exploring
        )
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Activity::Idle => "idle",
            Activity::Eating => "eating",
            Activity::Sleeping => "sleeping",
            Activity::Playing => "playing",
            Activity::Exploring => "exploring",
            Activity::Socialising => "socialising",
            Activity::Bathing => "bathing",
            Activity::Battling => "battling",
            Activity::Recovering => "recovering",
        }
    }
}

impl fmt::Display for Activity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The current activity and when it started.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivityState {
    current: Activity,
    since: Tick,
    /// `None` for activities that continue until something else interrupts.
    until: Option<Tick>,
}

impl ActivityState {
    #[must_use]
    pub const fn idle_since(now: Tick) -> Self {
        ActivityState {
            current: Activity::Idle,
            since: now,
            until: None,
        }
    }

    #[must_use]
    pub const fn current(&self) -> Activity {
        self.current
    }

    #[must_use]
    pub const fn since(&self) -> Tick {
        self.since
    }

    #[must_use]
    pub const fn until(&self) -> Option<Tick> {
        self.until
    }

    /// Starts an activity that runs for `duration`.
    pub const fn begin(&mut self, activity: Activity, now: Tick, duration: GameDuration) {
        self.current = activity;
        self.since = now;
        self.until = Some(now.saturating_add(duration));
    }

    /// Starts an activity with no fixed end.
    pub const fn begin_open_ended(&mut self, activity: Activity, now: Tick) {
        self.current = activity;
        self.since = now;
        self.until = None;
    }

    /// True once a timed activity's end tick has arrived.
    ///
    /// Open-ended activities are never finished; something has to interrupt
    /// them.
    #[must_use]
    pub const fn is_finished(&self, now: Tick) -> bool {
        match self.until {
            Some(end) => now.0 >= end.0,
            None => false,
        }
    }

    #[must_use]
    pub const fn elapsed(&self, now: Tick) -> GameDuration {
        now.since(self.since)
    }

    /// Returns to idle.
    pub const fn finish(&mut self, now: Tick) {
        self.current = Activity::Idle;
        self.since = now;
        self.until = None;
    }
}

impl Default for ActivityState {
    fn default() -> Self {
        Self::idle_since(Tick::ZERO)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_recovering_is_involuntary() {
        assert!(!Activity::Recovering.is_voluntary());
        for activity in Activity::ALL
            .into_iter()
            .filter(|a| *a != Activity::Recovering)
        {
            assert!(activity.is_voluntary(), "{activity} should be a choice");
        }
    }

    #[test]
    fn a_new_state_is_idle_and_open_ended() {
        let state = ActivityState::idle_since(Tick(10));
        assert_eq!(state.current(), Activity::Idle);
        assert_eq!(state.since(), Tick(10));
        assert_eq!(state.until(), None);
        assert!(
            !state.is_finished(Tick(1_000)),
            "an open-ended activity never self-terminates"
        );
    }

    #[test]
    fn a_timed_activity_finishes_on_its_end_tick() {
        let mut state = ActivityState::idle_since(Tick(0));
        state.begin(Activity::Eating, Tick(10), GameDuration::from_ticks(5));

        assert_eq!(state.until(), Some(Tick(15)));
        assert!(!state.is_finished(Tick(14)));
        assert!(
            state.is_finished(Tick(15)),
            "the end tick counts as finished"
        );
        assert!(state.is_finished(Tick(99)));
    }

    #[test]
    fn elapsed_measures_from_the_start() {
        let mut state = ActivityState::idle_since(Tick(0));
        state.begin_open_ended(Activity::Sleeping, Tick(100));
        assert_eq!(state.elapsed(Tick(160)), GameDuration::from_hours(1));
        assert_eq!(
            state.elapsed(Tick(50)),
            GameDuration::ZERO,
            "time cannot run backwards"
        );
    }

    #[test]
    fn finishing_returns_to_idle() {
        let mut state = ActivityState::idle_since(Tick(0));
        state.begin(Activity::Battling, Tick(5), GameDuration::from_ticks(3));
        state.finish(Tick(8));
        assert_eq!(state.current(), Activity::Idle);
        assert_eq!(state.since(), Tick(8));
        assert_eq!(state.until(), None);
    }
}
