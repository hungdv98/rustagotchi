//! Alleles — the variants that can occupy a locus.

use serde::{Deserialize, Serialize};

/// One variant at a locus, as an index into the species' allele table.
///
/// An index rather than a string: a pet carries one of these per locus per
/// parent, they are compared constantly during inheritance, and they land in
/// every save. The mapping back to a human-readable name lives in content.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct AlleleSymbol(pub u16);

/// How strongly an allele masks its partner. Higher wins.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Dominance(pub u8);

impl Dominance {
    /// Always masked by anything else.
    pub const RECESSIVE: Self = Dominance(0);
    /// The ordinary case.
    pub const STANDARD: Self = Dominance(128);
    /// Masks everything else.
    pub const DOMINANT: Self = Dominance(255);
}

/// A variant together with its dominance.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct Allele {
    pub symbol: AlleleSymbol,
    pub dominance: Dominance,
}

impl Allele {
    #[must_use]
    pub const fn new(symbol: AlleleSymbol, dominance: Dominance) -> Self {
        Allele { symbol, dominance }
    }

    /// An allele of ordinary dominance. The common case in content.
    #[must_use]
    pub const fn standard(symbol: u16) -> Self {
        Allele::new(AlleleSymbol(symbol), Dominance::STANDARD)
    }

    #[must_use]
    pub const fn recessive(symbol: u16) -> Self {
        Allele::new(AlleleSymbol(symbol), Dominance::RECESSIVE)
    }

    #[must_use]
    pub const fn dominant(symbol: u16) -> Self {
        Allele::new(AlleleSymbol(symbol), Dominance::DOMINANT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dominance_ranks_are_ordered() {
        assert!(Dominance::RECESSIVE < Dominance::STANDARD);
        assert!(Dominance::STANDARD < Dominance::DOMINANT);
    }

    #[test]
    fn constructors_set_the_dominance_they_name() {
        assert_eq!(Allele::recessive(1).dominance, Dominance::RECESSIVE);
        assert_eq!(Allele::standard(1).dominance, Dominance::STANDARD);
        assert_eq!(Allele::dominant(1).dominance, Dominance::DOMINANT);
    }

    #[test]
    fn alleles_sort_by_symbol_before_dominance() {
        // Ordering is relied on to break codominance ties deterministically.
        let mut alleles = [
            Allele::standard(3),
            Allele::dominant(1),
            Allele::recessive(2),
        ];
        alleles.sort();
        let symbols: Vec<u16> = alleles.iter().map(|allele| allele.symbol.0).collect();
        assert_eq!(symbols, [1, 2, 3]);
    }
}
