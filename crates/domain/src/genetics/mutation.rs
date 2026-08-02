//! Mutation value types.
//!
//! The engine that decides *whether* a mutation happens lives in
//! `simulation::breeding::mutation_engine` (Phase 5). What lives here is the
//! vocabulary: a bounded rate, and a record of what changed. Keeping the record
//! in the domain means a mutation can be reported as an event and shown in a
//! lineage view without the UI knowing how breeding works.

use serde::{Deserialize, Serialize};

use super::allele::{Allele, AlleleSymbol};
use crate::identifiers::GeneId;
use crate::validation::{ValidationError, require_range};

/// The chance of a mutation at one locus, in occurrences per million.
///
/// Parts per million rather than a fraction: mutation rates are small, authors
/// write them as whole numbers, and an integer keeps ADR 0006 intact all the way
/// from the content file to the roll.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct MutationRate(u32);

impl MutationRate {
    pub const DENOMINATOR: u32 = 1_000_000;
    pub const NEVER: Self = MutationRate(0);

    /// # Errors
    ///
    /// If `per_million` exceeds [`Self::DENOMINATOR`] — a rate above 100% is a
    /// content typo, and silently clamping it would hide the mistake.
    pub fn per_million(per_million: u32) -> Result<Self, ValidationError> {
        require_range(
            "mutation rate",
            i64::from(per_million),
            0,
            i64::from(Self::DENOMINATOR),
        )?;
        Ok(MutationRate(per_million))
    }

    #[must_use]
    pub const fn numerator(self) -> u32 {
        self.0
    }

    #[must_use]
    pub const fn is_impossible(self) -> bool {
        self.0 == 0
    }

    #[must_use]
    pub const fn is_certain(self) -> bool {
        self.0 >= Self::DENOMINATOR
    }
}

/// A mutation that occurred, recorded for lineage and events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mutation {
    pub gene: GeneId,
    pub from: AlleleSymbol,
    pub to: Allele,
}

impl Mutation {
    #[must_use]
    pub fn new(gene: GeneId, from: AlleleSymbol, to: Allele) -> Self {
        Mutation { gene, from, to }
    }

    /// A mutation that changed nothing is not worth recording; this catches the
    /// case where a roll picked the allele already present.
    #[must_use]
    pub fn is_silent(&self) -> bool {
        self.from == self.to.symbol
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rates_accept_the_whole_valid_range() {
        assert!(MutationRate::per_million(0).is_ok());
        assert!(MutationRate::per_million(1_000_000).is_ok());
        assert!(MutationRate::per_million(500).is_ok());
    }

    #[test]
    fn a_rate_above_one_hundred_percent_is_a_typo_not_a_clamp() {
        assert!(MutationRate::per_million(1_000_001).is_err());
    }

    #[test]
    fn the_extremes_are_recognised() {
        assert!(MutationRate::NEVER.is_impossible());
        assert!(MutationRate::per_million(1_000_000).unwrap().is_certain());
        assert!(!MutationRate::per_million(1).unwrap().is_impossible());
    }

    #[test]
    fn a_mutation_to_the_same_symbol_is_silent() {
        let gene = GeneId::new("coat_colour").unwrap();
        let noisy = Mutation::new(gene.clone(), AlleleSymbol(1), Allele::standard(2));
        let silent = Mutation::new(gene, AlleleSymbol(1), Allele::standard(1));
        assert!(!noisy.is_silent());
        assert!(silent.is_silent());
    }
}
