//! Elemental affinities and the effectiveness matrix.
//!
//! Pulled forward from Phase 5 because species and moves cannot be authored
//! without it. Only the affinity type and its matrix live here; damage
//! resolution stays in Phase 5.

use core::fmt;

use serde::{Deserialize, Serialize};

use crate::units::Fixed;

/// A move's or a species' elemental affinity.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum Element {
    /// No affinity. Never strong or weak against anything.
    #[default]
    Neutral,
    Ember,
    Aqua,
    Storm,
    Terra,
}

impl Element {
    pub const ALL: [Element; 5] = [
        Element::Neutral,
        Element::Ember,
        Element::Aqua,
        Element::Storm,
        Element::Terra,
    ];

    /// The four elements that participate in the matrix.
    pub const ELEMENTAL: [Element; 4] = [
        Element::Ember,
        Element::Aqua,
        Element::Storm,
        Element::Terra,
    ];

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Element::Neutral => "neutral",
            Element::Ember => "ember",
            Element::Aqua => "aqua",
            Element::Storm => "storm",
            Element::Terra => "terra",
        }
    }

    /// What this element is strong against, or `None` for [`Element::Neutral`].
    ///
    /// A single cycle — ember → terra → storm → aqua → ember — rather than a
    /// hand-authored table. A cycle is provably balanced: every element beats
    /// exactly one and loses to exactly one, so no species can be strictly
    /// dominant and the nightly 100,000-battle usage check has a fair baseline
    /// to measure against.
    #[must_use]
    pub const fn strong_against(self) -> Option<Element> {
        match self {
            Element::Ember => Some(Element::Terra),
            Element::Terra => Some(Element::Storm),
            Element::Storm => Some(Element::Aqua),
            Element::Aqua => Some(Element::Ember),
            Element::Neutral => None,
        }
    }

    #[must_use]
    pub const fn weak_against(self) -> Option<Element> {
        match self {
            Element::Ember => Some(Element::Aqua),
            Element::Aqua => Some(Element::Storm),
            Element::Storm => Some(Element::Terra),
            Element::Terra => Some(Element::Ember),
            Element::Neutral => None,
        }
    }

    /// The damage multiplier when `self` attacks `defender`.
    ///
    /// `2.0` when strong, `0.5` when weak, `1.0` otherwise. Exact in Q16.16, so
    /// the same matchup always produces the same damage.
    #[must_use]
    pub fn effectiveness_against(self, defender: Element) -> Fixed {
        if self.strong_against() == Some(defender) {
            Fixed::from_int(2)
        } else if self.weak_against() == Some(defender) {
            Fixed::from_ratio(1, 2)
        } else {
            Fixed::ONE
        }
    }
}

impl fmt::Display for Element {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutral_is_outside_the_cycle() {
        assert_eq!(Element::Neutral.strong_against(), None);
        assert_eq!(Element::Neutral.weak_against(), None);
        for element in Element::ALL {
            assert_eq!(Element::Neutral.effectiveness_against(element), Fixed::ONE);
            assert_eq!(element.effectiveness_against(Element::Neutral), Fixed::ONE);
        }
    }

    #[test]
    fn the_cycle_is_consistent_in_both_directions() {
        // If A is strong against B, then B must be weak against A. Stated once
        // here so the two tables cannot drift apart.
        for attacker in Element::ELEMENTAL {
            let victim = attacker.strong_against().expect("elemental");
            assert_eq!(
                victim.weak_against(),
                Some(attacker),
                "{attacker} vs {victim}"
            );
        }
    }

    #[test]
    fn every_element_beats_exactly_one_and_loses_to_exactly_one() {
        for element in Element::ELEMENTAL {
            let beats = Element::ELEMENTAL
                .into_iter()
                .filter(|other| element.effectiveness_against(*other) > Fixed::ONE)
                .count();
            let loses = Element::ELEMENTAL
                .into_iter()
                .filter(|other| element.effectiveness_against(*other) < Fixed::ONE)
                .count();
            assert_eq!(
                beats, 1,
                "{element} should be strong against exactly one element"
            );
            assert_eq!(
                loses, 1,
                "{element} should be weak against exactly one element"
            );
        }
    }

    #[test]
    fn no_element_is_strong_or_weak_against_itself() {
        for element in Element::ALL {
            assert_eq!(element.effectiveness_against(element), Fixed::ONE);
        }
    }

    #[test]
    fn the_cycle_visits_every_element_before_returning() {
        let mut visited = vec![Element::Ember];
        let mut current = Element::Ember;
        for _ in 0..Element::ELEMENTAL.len() - 1 {
            current = current.strong_against().expect("elemental");
            assert!(
                !visited.contains(&current),
                "the cycle must not close early"
            );
            visited.push(current);
        }
        assert_eq!(
            current.strong_against(),
            Some(Element::Ember),
            "the cycle must close"
        );
        assert_eq!(visited.len(), Element::ELEMENTAL.len());
    }

    #[test]
    fn multipliers_are_exact() {
        assert_eq!(
            Element::Ember.effectiveness_against(Element::Terra),
            Fixed::from_int(2)
        );
        assert_eq!(
            Element::Ember.effectiveness_against(Element::Aqua),
            Fixed::from_ratio(1, 2)
        );
        assert_eq!(
            Element::Ember.effectiveness_against(Element::Storm),
            Fixed::ONE
        );
    }
}
