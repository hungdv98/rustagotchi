//! Gene pairs and how they express.

use serde::{Deserialize, Serialize};

use super::allele::Allele;

/// The diploid pair a pet carries at one locus.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct GenePair {
    pub maternal: Allele,
    pub paternal: Allele,
}

/// What actually shows at a locus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Expression {
    /// One allele masks the other, or both are the same.
    Dominant(Allele),
    /// Equal dominance and different symbols: both contribute.
    ///
    /// Always ordered by [`Allele`]'s own ordering, never by which parent
    /// supplied which — see [`GenePair::expressed`].
    Codominant(Allele, Allele),
}

impl GenePair {
    #[must_use]
    pub const fn new(maternal: Allele, paternal: Allele) -> Self {
        GenePair { maternal, paternal }
    }

    /// Both copies identical.
    #[must_use]
    pub const fn homozygous(allele: Allele) -> Self {
        GenePair {
            maternal: allele,
            paternal: allele,
        }
    }

    #[must_use]
    pub fn is_homozygous(&self) -> bool {
        self.maternal.symbol == self.paternal.symbol
    }

    /// What this locus expresses.
    ///
    /// Higher dominance masks lower. On a tie the pair is codominant, and the
    /// two alleles are returned in sorted order.
    ///
    /// That sort is not cosmetic. Which parent is "maternal" is an accident of
    /// how the breeding call was written; if the expression depended on it, the
    /// same cross would produce different phenotypes depending on argument
    /// order, and offline/live equivalence would fail in a way that looks like a
    /// genetics bug rather than an ordering bug.
    #[must_use]
    pub fn expressed(&self) -> Expression {
        use core::cmp::Ordering;

        match self.maternal.dominance.cmp(&self.paternal.dominance) {
            Ordering::Greater => Expression::Dominant(self.maternal),
            Ordering::Less => Expression::Dominant(self.paternal),
            Ordering::Equal if self.maternal.symbol == self.paternal.symbol => {
                Expression::Dominant(self.maternal)
            }
            Ordering::Equal => {
                let (low, high) = if self.maternal <= self.paternal {
                    (self.maternal, self.paternal)
                } else {
                    (self.paternal, self.maternal)
                };
                Expression::Codominant(low, high)
            }
        }
    }

    /// The two copies, in a fixed order independent of parentage.
    #[must_use]
    pub fn sorted_alleles(&self) -> [Allele; 2] {
        if self.maternal <= self.paternal {
            [self.maternal, self.paternal]
        } else {
            [self.paternal, self.maternal]
        }
    }

    /// True if `symbol` is present on either copy, expressed or not.
    ///
    /// A carrier of a masked recessive still passes it on, which is the whole
    /// point of tracking both copies.
    #[must_use]
    pub fn carries(&self, allele: Allele) -> bool {
        self.maternal.symbol == allele.symbol || self.paternal.symbol == allele.symbol
    }
}

impl Expression {
    /// The alleles on show.
    #[must_use]
    pub fn alleles(&self) -> smallvec::SmallVec<[Allele; 2]> {
        match self {
            Expression::Dominant(allele) => smallvec::smallvec![*allele],
            Expression::Codominant(first, second) => smallvec::smallvec![*first, *second],
        }
    }

    #[must_use]
    pub const fn is_codominant(&self) -> bool {
        matches!(self, Expression::Codominant(_, _))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_more_dominant_allele_masks_the_other() {
        let pair = GenePair::new(Allele::dominant(1), Allele::recessive(2));
        assert_eq!(pair.expressed(), Expression::Dominant(Allele::dominant(1)));
    }

    #[test]
    fn a_homozygous_pair_expresses_its_single_allele() {
        let pair = GenePair::homozygous(Allele::standard(4));
        assert!(pair.is_homozygous());
        assert_eq!(pair.expressed(), Expression::Dominant(Allele::standard(4)));
    }

    #[test]
    fn equal_dominance_with_different_symbols_is_codominant() {
        let pair = GenePair::new(Allele::standard(1), Allele::standard(2));
        assert!(pair.expressed().is_codominant());
        assert_eq!(pair.expressed().alleles().len(), 2);
    }

    #[test]
    fn expression_does_not_depend_on_which_parent_supplied_which() {
        // The determinism property: swapping the parents must not change the
        // phenotype, for every combination of dominance.
        let candidates = [
            Allele::recessive(1),
            Allele::standard(1),
            Allele::standard(2),
            Allele::dominant(2),
            Allele::dominant(3),
        ];
        for maternal in candidates {
            for paternal in candidates {
                let forward = GenePair::new(maternal, paternal).expressed();
                let reversed = GenePair::new(paternal, maternal).expressed();
                assert_eq!(
                    forward, reversed,
                    "swapping parents changed {maternal:?}/{paternal:?}"
                );
            }
        }
    }

    #[test]
    fn sorted_alleles_are_parent_order_independent() {
        let first = GenePair::new(Allele::standard(5), Allele::standard(2));
        let second = GenePair::new(Allele::standard(2), Allele::standard(5));
        assert_eq!(first.sorted_alleles(), second.sorted_alleles());
    }

    #[test]
    fn a_masked_recessive_is_still_carried() {
        let pair = GenePair::new(Allele::dominant(1), Allele::recessive(9));
        assert!(
            pair.carries(Allele::recessive(9)),
            "carriers must be detectable"
        );
        assert_eq!(pair.expressed(), Expression::Dominant(Allele::dominant(1)));
    }

    #[test]
    fn a_pair_survives_a_serde_round_trip() {
        let pair = GenePair::new(Allele::standard(1), Allele::recessive(2));
        let encoded = serde_json::to_string(&pair).expect("serialize");
        assert_eq!(
            serde_json::from_str::<GenePair>(&encoded).expect("deserialize"),
            pair
        );
    }
}
