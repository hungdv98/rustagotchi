//! The life stage ladder.

use core::fmt;

use serde::{Deserialize, Serialize};

use crate::error::DomainError;

/// Where a pet is in its life.
///
/// Ordered and monotonic: a pet only ever moves forward through these. That is
/// enforced by [`LifeStage::advance_to`] rather than left to callers, because a
/// stage regression corrupts progression, breeding eligibility and evolution
/// reachability all at once, and does so silently.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub enum LifeStage {
    /// Not yet hatched. Has genetics but no needs.
    #[default]
    Egg,
    Hatchling,
    Child,
    Teen,
    Adult,
    Elder,
}

impl LifeStage {
    /// Every stage, in order. Iterate this rather than writing a `match` that
    /// silently forgets a variant when one is added.
    pub const ORDER: [LifeStage; 6] = [
        LifeStage::Egg,
        LifeStage::Hatchling,
        LifeStage::Child,
        LifeStage::Teen,
        LifeStage::Adult,
        LifeStage::Elder,
    ];

    #[must_use]
    pub const fn index(self) -> u8 {
        self as u8
    }

    /// The next stage, or `None` at the end of the ladder.
    #[must_use]
    pub const fn next(self) -> Option<Self> {
        match self {
            LifeStage::Egg => Some(LifeStage::Hatchling),
            LifeStage::Hatchling => Some(LifeStage::Child),
            LifeStage::Child => Some(LifeStage::Teen),
            LifeStage::Teen => Some(LifeStage::Adult),
            LifeStage::Adult => Some(LifeStage::Elder),
            LifeStage::Elder => None,
        }
    }

    /// True once the pet has hatched and has needs to simulate.
    #[must_use]
    pub const fn is_hatched(self) -> bool {
        !matches!(self, LifeStage::Egg)
    }

    /// Whether the stage is old enough to breed at all.
    ///
    /// A coarse gate only. Species-specific eligibility — cooldowns, health,
    /// lineage — is content-driven and lives in `domain::breeding`.
    #[must_use]
    pub const fn can_breed(self) -> bool {
        matches!(self, LifeStage::Adult | LifeStage::Elder)
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            LifeStage::Egg => "egg",
            LifeStage::Hatchling => "hatchling",
            LifeStage::Child => "child",
            LifeStage::Teen => "teen",
            LifeStage::Adult => "adult",
            LifeStage::Elder => "elder",
        }
    }

    /// Moves forward to `target`.
    ///
    /// # Errors
    ///
    /// [`DomainError::LifeStageRegression`] if `target` is earlier than `self`.
    /// Advancing to the same stage is a no-op rather than an error, so that a
    /// system which recomputes the stage every tick does not have to special-case
    /// the common path.
    pub fn advance_to(&mut self, target: LifeStage) -> Result<(), DomainError> {
        if target < *self {
            return Err(DomainError::LifeStageRegression {
                from: self.name(),
                to: target.name(),
            });
        }
        *self = target;
        Ok(())
    }
}

impl fmt::Display for LifeStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_matches_the_declared_ordering() {
        for pair in LifeStage::ORDER.windows(2) {
            assert!(
                pair[0] < pair[1],
                "{:?} should precede {:?}",
                pair[0],
                pair[1]
            );
        }
    }

    #[test]
    fn next_walks_the_whole_ladder_exactly_once() {
        let mut stage = LifeStage::Egg;
        let mut visited = vec![stage];
        while let Some(following) = stage.next() {
            stage = following;
            visited.push(stage);
        }
        assert_eq!(visited, LifeStage::ORDER.to_vec());
    }

    #[test]
    fn the_ladder_terminates() {
        assert_eq!(LifeStage::Elder.next(), None);
    }

    #[test]
    fn indices_are_contiguous_from_zero() {
        for (position, stage) in LifeStage::ORDER.iter().enumerate() {
            assert_eq!(stage.index(), position as u8);
        }
    }

    #[test]
    fn only_eggs_are_unhatched() {
        assert!(!LifeStage::Egg.is_hatched());
        for stage in LifeStage::ORDER.iter().filter(|s| **s != LifeStage::Egg) {
            assert!(stage.is_hatched(), "{stage} should count as hatched");
        }
    }

    #[test]
    fn breeding_is_gated_to_mature_stages() {
        assert!(LifeStage::Adult.can_breed());
        assert!(LifeStage::Elder.can_breed());
        for stage in [
            LifeStage::Egg,
            LifeStage::Hatchling,
            LifeStage::Child,
            LifeStage::Teen,
        ] {
            assert!(!stage.can_breed(), "{stage} must not be able to breed");
        }
    }

    #[test]
    fn advancing_forward_succeeds() {
        let mut stage = LifeStage::Child;
        stage.advance_to(LifeStage::Adult).expect("forward advance");
        assert_eq!(stage, LifeStage::Adult);
    }

    #[test]
    fn advancing_to_the_current_stage_is_a_no_op() {
        let mut stage = LifeStage::Teen;
        stage
            .advance_to(LifeStage::Teen)
            .expect("same-stage advance");
        assert_eq!(stage, LifeStage::Teen);
    }

    #[test]
    fn regression_is_refused_and_leaves_the_stage_untouched() {
        let mut stage = LifeStage::Adult;
        let error = stage.advance_to(LifeStage::Child).unwrap_err();
        assert!(matches!(error, DomainError::LifeStageRegression { .. }));
        assert_eq!(stage, LifeStage::Adult, "a refused advance must not mutate");
    }

    #[test]
    fn stages_survive_a_serde_round_trip() {
        for stage in LifeStage::ORDER {
            let encoded = serde_json::to_string(&stage).expect("serialize");
            let decoded: LifeStage = serde_json::from_str(&encoded).expect("deserialize");
            assert_eq!(stage, decoded);
        }
    }
}
