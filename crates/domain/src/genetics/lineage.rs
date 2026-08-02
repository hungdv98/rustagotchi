//! Ancestry and relatedness.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::identifiers::PetId;
use crate::units::Milli;

/// Who a pet's parents were.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Parentage {
    /// No recorded parents — a starter pet or one from a content pack.
    #[default]
    Founder,
    Bred {
        parents: [PetId; 2],
    },
}

impl Parentage {
    #[must_use]
    pub const fn parents(&self) -> Option<&[PetId; 2]> {
        match self {
            Parentage::Founder => None,
            Parentage::Bred { parents } => Some(parents),
        }
    }

    #[must_use]
    pub const fn is_founder(&self) -> bool {
        matches!(self, Parentage::Founder)
    }
}

/// A pet's ancestors, grouped by how many generations back they are.
///
/// Bounded at [`Ancestry::TRACKED_GENERATIONS`]. Unbounded ancestry would grow
/// without limit in a long-running save, and relatedness beyond a few
/// generations is not something a player can perceive anyway.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ancestry {
    /// `generations[0]` is the parents, `[1]` the grandparents, and so on.
    generations: Vec<BTreeSet<PetId>>,
}

impl Ancestry {
    /// How far back ancestors are remembered.
    pub const TRACKED_GENERATIONS: usize = 5;

    /// The scale returned by [`Self::shared_ancestry`]: `Milli(1_000)` is 1.0.
    pub const FULLY_RELATED: Milli = Milli::ONE;

    #[must_use]
    pub fn founder() -> Self {
        Ancestry {
            generations: Vec::new(),
        }
    }

    /// Builds the ancestry of a child of two pets.
    ///
    /// The parents themselves become generation 0; each parent's generation `n`
    /// becomes the child's generation `n + 1`, truncated at the tracking limit.
    #[must_use]
    pub fn from_parents(
        first_parent: PetId,
        first_ancestry: &Ancestry,
        second_parent: PetId,
        second_ancestry: &Ancestry,
    ) -> Self {
        let mut generations: Vec<BTreeSet<PetId>> = Vec::new();
        generations.push([first_parent, second_parent].into_iter().collect());

        for depth in 0..Self::TRACKED_GENERATIONS.saturating_sub(1) {
            let mut merged = BTreeSet::new();
            if let Some(ancestors) = first_ancestry.generations.get(depth) {
                merged.extend(ancestors.iter().copied());
            }
            if let Some(ancestors) = second_ancestry.generations.get(depth) {
                merged.extend(ancestors.iter().copied());
            }
            if merged.is_empty() {
                break;
            }
            generations.push(merged);
        }

        Ancestry { generations }
    }

    #[must_use]
    pub fn parents(&self) -> Option<&BTreeSet<PetId>> {
        self.generations.first()
    }

    #[must_use]
    pub fn is_ancestor(&self, candidate: PetId) -> bool {
        self.generations
            .iter()
            .any(|generation| generation.contains(&candidate))
    }

    #[must_use]
    pub fn depth(&self) -> usize {
        self.generations.len()
    }

    /// Relatedness in thousandths, `0..=1_000`.
    ///
    /// Each ancestor shared at generation `g` (0 = parents) contributes
    /// `(1/2)^(2(g+1))`, which is the standard coefficient of relationship: a
    /// path up to the common ancestor and back down is `2(g+1)` meioses. That
    /// reproduces the textbook figures —
    ///
    /// | Relationship | Shared | Depth | Score |
    /// |---|---|---|---|
    /// | full siblings | 2 parents | 0 | `0.500` |
    /// | half siblings | 1 parent | 0 | `0.250` |
    /// | first cousins | 2 grandparents | 1 | `0.125` |
    /// | unrelated | — | — | `0.000` |
    ///
    /// **It remains an approximation.** Only [`Self::TRACKED_GENERATIONS`] are
    /// remembered, an ancestor appearing at several depths is counted at each,
    /// and the result is capped at [`Self::FULLY_RELATED`] — so on a deliberately
    /// inbred line it overstates. That is the right direction to be wrong in for
    /// a breeding penalty, but it is why this is not called Wright's `F`: nobody
    /// should reach for it expecting a coefficient of inbreeding.
    #[must_use]
    pub fn shared_ancestry(&self, other: &Ancestry) -> Milli {
        let mut total = Milli::ZERO;

        for depth in 0..self.generations.len().min(other.generations.len()) {
            let shared = self.generations[depth]
                .intersection(&other.generations[depth])
                .count();
            if shared == 0 {
                continue;
            }
            // (1/2)^(2(depth+1)) == 1 / 4^(depth+1), in thousandths, truncating.
            let divisor = 1i32 << (2 * (depth as u32 + 1));
            let contribution = Milli::ONE.scale(shared as i32, divisor);
            total = total.saturating_add(contribution);
        }

        total.clamp_to(Milli::ZERO, Self::FULLY_RELATED)
    }

    /// True if either pet appears in the other's ancestry, at any depth.
    ///
    /// Direct-line breeding is refused outright rather than merely penalised,
    /// which is why this is separate from [`Self::shared_ancestry`].
    #[must_use]
    pub fn is_direct_line(&self, own_id: PetId, other: &Ancestry, other_id: PetId) -> bool {
        self.is_ancestor(other_id) || other.is_ancestor(own_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::WorldSeed;
    use crate::time::Tick;

    fn pet(ordinal: u64) -> PetId {
        PetId::derive(WorldSeed(1), Tick::ZERO, ordinal)
    }

    /// Two pets bred from the same two parents.
    fn siblings() -> (Ancestry, Ancestry) {
        let (mother, father) = (pet(1), pet(2));
        let founder = Ancestry::founder();
        (
            Ancestry::from_parents(mother, &founder, father, &founder),
            Ancestry::from_parents(mother, &founder, father, &founder),
        )
    }

    #[test]
    fn a_founder_has_no_ancestors() {
        let founder = Ancestry::founder();
        assert_eq!(founder.depth(), 0);
        assert!(founder.parents().is_none());
        assert!(!founder.is_ancestor(pet(1)));
    }

    #[test]
    fn parentage_reports_its_parents() {
        assert!(Parentage::Founder.is_founder());
        assert!(Parentage::Founder.parents().is_none());

        let bred = Parentage::Bred {
            parents: [pet(1), pet(2)],
        };
        assert_eq!(bred.parents(), Some(&[pet(1), pet(2)]));
    }

    #[test]
    fn a_child_records_both_parents_at_generation_zero() {
        let founder = Ancestry::founder();
        let child = Ancestry::from_parents(pet(1), &founder, pet(2), &founder);
        assert_eq!(child.depth(), 1);
        assert!(child.is_ancestor(pet(1)));
        assert!(child.is_ancestor(pet(2)));
        assert!(!child.is_ancestor(pet(3)));
    }

    #[test]
    fn grandparents_move_back_one_generation() {
        let founder = Ancestry::founder();
        let mother = Ancestry::from_parents(pet(10), &founder, pet(11), &founder);
        let father = Ancestry::from_parents(pet(12), &founder, pet(13), &founder);
        let child = Ancestry::from_parents(pet(1), &mother, pet(2), &father);

        assert_eq!(child.depth(), 2);
        assert!(child.parents().expect("parents").contains(&pet(1)));
        for grandparent in [10, 11, 12, 13] {
            assert!(
                child.is_ancestor(pet(grandparent)),
                "grandparent {grandparent} lost"
            );
        }
    }

    #[test]
    fn ancestry_is_bounded() {
        // Breed a long line and confirm the record stops growing.
        let mut ancestry = Ancestry::founder();
        for generation in 0..20u64 {
            ancestry = Ancestry::from_parents(
                pet(generation * 2),
                &ancestry,
                pet(generation * 2 + 1),
                &ancestry,
            );
        }
        assert_eq!(ancestry.depth(), Ancestry::TRACKED_GENERATIONS);
    }

    #[test]
    fn strangers_are_unrelated() {
        let founder = Ancestry::founder();
        let first = Ancestry::from_parents(pet(1), &founder, pet(2), &founder);
        let second = Ancestry::from_parents(pet(3), &founder, pet(4), &founder);
        assert_eq!(first.shared_ancestry(&second), Milli::ZERO);
    }

    #[test]
    fn full_siblings_score_one_half() {
        let (first, second) = siblings();
        assert_eq!(first.shared_ancestry(&second), Milli(500));
    }

    #[test]
    fn half_siblings_score_one_quarter() {
        let founder = Ancestry::founder();
        let shared_parent = pet(1);
        let first = Ancestry::from_parents(shared_parent, &founder, pet(2), &founder);
        let second = Ancestry::from_parents(shared_parent, &founder, pet(3), &founder);
        assert_eq!(first.shared_ancestry(&second), Milli(250));
    }

    #[test]
    fn first_cousins_score_one_eighth() {
        let founder = Ancestry::founder();
        // Two siblings each have a child with an unrelated partner.
        let (sibling_a, sibling_b) = siblings();
        let cousin_a = Ancestry::from_parents(pet(10), &sibling_a, pet(20), &founder);
        let cousin_b = Ancestry::from_parents(pet(11), &sibling_b, pet(21), &founder);
        assert_eq!(cousin_a.shared_ancestry(&cousin_b), Milli(125));
    }

    #[test]
    fn relatedness_decreases_with_distance() {
        // The ordering players expect, asserted as a chain rather than as three
        // independent constants.
        let founder = Ancestry::founder();
        let (full_a, full_b) = siblings();
        let full = full_a.shared_ancestry(&full_b);

        let half = Ancestry::from_parents(pet(1), &founder, pet(2), &founder)
            .shared_ancestry(&Ancestry::from_parents(pet(1), &founder, pet(3), &founder));

        let cousins = Ancestry::from_parents(pet(10), &full_a, pet(20), &founder)
            .shared_ancestry(&Ancestry::from_parents(pet(11), &full_b, pet(21), &founder));

        let strangers =
            Ancestry::from_parents(pet(30), &founder, pet(31), &founder).shared_ancestry(
                &Ancestry::from_parents(pet(32), &founder, pet(33), &founder),
            );

        assert!(full > half, "full siblings outrank half siblings");
        assert!(half > cousins, "half siblings outrank cousins");
        assert!(cousins > strangers, "cousins outrank strangers");
        assert_eq!(strangers, Milli::ZERO);
    }

    #[test]
    fn relatedness_is_symmetric() {
        let (first, second) = siblings();
        assert_eq!(
            first.shared_ancestry(&second),
            second.shared_ancestry(&first)
        );
    }

    #[test]
    fn relatedness_never_exceeds_the_cap() {
        // A deliberately incestuous line: every generation reuses the same pair.
        let mut ancestry = Ancestry::founder();
        for _ in 0..6 {
            ancestry = Ancestry::from_parents(pet(1), &ancestry, pet(2), &ancestry);
        }
        assert!(ancestry.shared_ancestry(&ancestry) <= Ancestry::FULLY_RELATED);
    }

    #[test]
    fn direct_line_breeding_is_detectable() {
        let founder = Ancestry::founder();
        let parent_id = pet(1);
        let child = Ancestry::from_parents(parent_id, &founder, pet(2), &founder);
        let child_id = pet(100);

        assert!(child.is_direct_line(child_id, &founder, parent_id));
        assert!(founder.is_direct_line(parent_id, &child, child_id));

        let unrelated = Ancestry::from_parents(pet(8), &founder, pet(9), &founder);
        assert!(!child.is_direct_line(child_id, &unrelated, pet(200)));
    }
}
