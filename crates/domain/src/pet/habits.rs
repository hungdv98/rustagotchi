//! Habits — what a pet has got used to doing.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::activity::Activity;
use crate::units::Milli;

/// How ingrained a pet's habits are.
///
/// Keyed by [`Activity`] in a `BTreeMap`, so iteration — and therefore anything
/// that picks a "favourite" — is deterministic.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Habits {
    strengths: BTreeMap<Activity, Milli>,
}

impl Habits {
    /// The ceiling on any one habit.
    pub const MAX_STRENGTH: Milli = Milli(100_000);

    /// Below this a habit is dropped entirely, so decayed habits do not
    /// accumulate as a long tail of near-zero entries in every save.
    pub const FORGET_BELOW: Milli = Milli(100);

    #[must_use]
    pub fn new() -> Self {
        Habits::default()
    }

    #[must_use]
    pub fn strength_of(&self, activity: Activity) -> Milli {
        self.strengths
            .get(&activity)
            .copied()
            .unwrap_or(Milli::ZERO)
    }

    /// Strengthens the habit for `activity`.
    ///
    /// Involuntary activities are ignored — a pet laid up recovering is not
    /// acquiring a taste for illness.
    pub fn reinforce(&mut self, activity: Activity, amount: Milli) {
        if !activity.is_voluntary() || amount.0 <= 0 {
            return;
        }
        let strengthened = self
            .strength_of(activity)
            .saturating_add(amount)
            .clamp_to(Milli::ZERO, Self::MAX_STRENGTH);
        self.strengths.insert(activity, strengthened);
    }

    /// Weakens every habit, dropping any that fall below [`Self::FORGET_BELOW`].
    pub fn decay_all(&mut self, amount: Milli) {
        if amount.0 <= 0 {
            return;
        }
        self.strengths.retain(|_, strength| {
            *strength = strength.saturating_sub(amount);
            *strength >= Self::FORGET_BELOW
        });
    }

    /// The strongest habit, or `None` if the pet has none.
    ///
    /// Ties break toward the earlier [`Activity`] variant.
    #[must_use]
    pub fn strongest(&self) -> Option<(Activity, Milli)> {
        let mut best: Option<(Activity, Milli)> = None;
        for (activity, strength) in &self.strengths {
            match best {
                Some((_, current)) if *strength <= current => {}
                _ => best = Some((*activity, *strength)),
            }
        }
        best
    }

    pub fn iter(&self) -> impl Iterator<Item = (Activity, Milli)> + '_ {
        self.strengths
            .iter()
            .map(|(activity, strength)| (*activity, *strength))
    }

    #[must_use]
    pub fn count(&self) -> usize {
        self.strengths.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.strengths.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_pet_has_no_habits() {
        let habits = Habits::new();
        assert!(habits.is_empty());
        assert_eq!(habits.strength_of(Activity::Playing), Milli::ZERO);
        assert_eq!(habits.strongest(), None);
    }

    #[test]
    fn reinforcement_accumulates_and_caps() {
        let mut habits = Habits::new();
        habits.reinforce(Activity::Playing, Milli(1_000));
        habits.reinforce(Activity::Playing, Milli(500));
        assert_eq!(habits.strength_of(Activity::Playing), Milli(1_500));

        habits.reinforce(Activity::Playing, Milli(i32::MAX));
        assert_eq!(habits.strength_of(Activity::Playing), Habits::MAX_STRENGTH);
    }

    #[test]
    fn involuntary_activities_do_not_become_habits() {
        let mut habits = Habits::new();
        habits.reinforce(Activity::Recovering, Milli(50_000));
        assert!(habits.is_empty(), "being ill is not a hobby");
    }

    #[test]
    fn negative_and_zero_reinforcement_is_ignored() {
        let mut habits = Habits::new();
        habits.reinforce(Activity::Playing, Milli(0));
        habits.reinforce(Activity::Playing, Milli(-500));
        assert!(habits.is_empty());
    }

    #[test]
    fn decay_weakens_and_eventually_forgets() {
        let mut habits = Habits::new();
        habits.reinforce(Activity::Playing, Milli(1_000));
        habits.reinforce(Activity::Eating, Milli(10_000));

        habits.decay_all(Milli(500));
        assert_eq!(habits.strength_of(Activity::Playing), Milli(500));
        assert_eq!(habits.strength_of(Activity::Eating), Milli(9_500));

        habits.decay_all(Milli(450));
        assert_eq!(
            habits.count(),
            1,
            "a habit below the floor is dropped, not kept at zero"
        );
        assert_eq!(habits.strength_of(Activity::Playing), Milli::ZERO);
    }

    #[test]
    fn decay_never_leaves_a_negative_strength() {
        let mut habits = Habits::new();
        habits.reinforce(Activity::Exploring, Milli(1_000));
        habits.decay_all(Milli(i32::MAX));
        assert!(habits.is_empty());
    }

    #[test]
    fn the_strongest_habit_is_reported_and_ties_break_deterministically() {
        let mut habits = Habits::new();
        habits.reinforce(Activity::Playing, Milli(1_000));
        habits.reinforce(Activity::Eating, Milli(5_000));
        assert_eq!(habits.strongest(), Some((Activity::Eating, Milli(5_000))));

        // Equal strengths: the earlier Activity variant wins, every time.
        let mut tied = Habits::new();
        tied.reinforce(Activity::Playing, Milli(1_000));
        tied.reinforce(Activity::Sleeping, Milli(1_000));
        assert_eq!(
            tied.strongest().map(|(activity, _)| activity),
            Some(Activity::Sleeping),
            "Sleeping precedes Playing, so it wins the tie regardless of insertion order"
        );
    }

    #[test]
    fn iteration_is_ordered_by_activity() {
        let mut habits = Habits::new();
        habits.reinforce(Activity::Bathing, Milli(1_000));
        habits.reinforce(Activity::Eating, Milli(1_000));
        let order: Vec<Activity> = habits.iter().map(|(activity, _)| activity).collect();
        assert_eq!(order, [Activity::Eating, Activity::Bathing]);
    }
}
