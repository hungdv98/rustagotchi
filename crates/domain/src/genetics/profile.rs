//! A pet's full complement of gene pairs.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::allele::Allele;
use super::gene::GenePair;
use crate::error::DomainError;
use crate::identifiers::GeneId;

/// Every locus a pet carries, keyed by gene.
///
/// `BTreeMap`, not `HashMap`: iteration order is part of the determinism
/// contract. Phenotype computation, inheritance and any digest over genetics all
/// walk this map, and `HashMap` would make the order vary between runs.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneticProfile {
    loci: BTreeMap<GeneId, GenePair>,
}

impl GeneticProfile {
    #[must_use]
    pub fn new() -> Self {
        GeneticProfile {
            loci: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn from_loci(loci: impl IntoIterator<Item = (GeneId, GenePair)>) -> Self {
        GeneticProfile {
            loci: loci.into_iter().collect(),
        }
    }

    #[must_use]
    pub fn pair(&self, gene: &GeneId) -> Option<&GenePair> {
        self.loci.get(gene)
    }

    pub fn insert(&mut self, gene: GeneId, pair: GenePair) -> Option<GenePair> {
        self.loci.insert(gene, pair)
    }

    /// Loci in ascending gene order, always.
    pub fn iter(&self) -> impl Iterator<Item = (&GeneId, &GenePair)> {
        self.loci.iter()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.loci.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.loci.is_empty()
    }

    /// True if any locus carries `allele`, expressed or masked.
    #[must_use]
    pub fn carries(&self, allele: Allele) -> bool {
        self.loci.values().any(|pair| pair.carries(allele))
    }

    #[must_use]
    pub fn homozygous_loci(&self) -> usize {
        self.loci
            .values()
            .filter(|pair| pair.is_homozygous())
            .count()
    }

    /// Checks the profile against the loci its species declares.
    ///
    /// A pet missing a locus, or carrying one its species does not define, is a
    /// content or migration bug. Catching it here means the phenotype code
    /// downstream can index without an `Option`, and a bad content pack fails at
    /// load rather than as a missing trait three hours into a save.
    ///
    /// # Errors
    ///
    /// [`DomainError::GeneticSchemaMismatch`] listing what is missing and what
    /// is unexpected — both, in one pass, because fixing them one at a time is
    /// miserable.
    pub fn check_against_schema(&self, expected: &[GeneId]) -> Result<(), DomainError> {
        let expected_set: BTreeSet<&GeneId> = expected.iter().collect();

        let missing: Vec<String> = expected
            .iter()
            .filter(|gene| !self.loci.contains_key(*gene))
            .map(|gene| gene.as_str().to_owned())
            .collect();

        let unexpected: Vec<String> = self
            .loci
            .keys()
            .filter(|gene| !expected_set.contains(*gene))
            .map(|gene| gene.as_str().to_owned())
            .collect();

        if missing.is_empty() && unexpected.is_empty() {
            Ok(())
        } else {
            Err(DomainError::GeneticSchemaMismatch {
                missing,
                unexpected,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genetics::allele::Allele;

    fn gene(name: &str) -> GeneId {
        GeneId::new(name).expect("valid gene id")
    }

    fn sample() -> GeneticProfile {
        GeneticProfile::from_loci([
            (
                gene("coat_colour"),
                GenePair::new(Allele::dominant(1), Allele::recessive(2)),
            ),
            (gene("size"), GenePair::homozygous(Allele::standard(3))),
        ])
    }

    #[test]
    fn loci_round_trip_through_the_profile() {
        let profile = sample();
        assert_eq!(profile.len(), 2);
        assert!(profile.pair(&gene("coat_colour")).is_some());
        assert!(profile.pair(&gene("nonexistent")).is_none());
    }

    #[test]
    fn iteration_is_in_gene_order_regardless_of_insertion_order() {
        let forward = GeneticProfile::from_loci([
            (gene("aaa"), GenePair::homozygous(Allele::standard(1))),
            (gene("zzz"), GenePair::homozygous(Allele::standard(2))),
        ]);
        let backward = GeneticProfile::from_loci([
            (gene("zzz"), GenePair::homozygous(Allele::standard(2))),
            (gene("aaa"), GenePair::homozygous(Allele::standard(1))),
        ]);

        let names = |profile: &GeneticProfile| {
            profile
                .iter()
                .map(|(id, _)| id.as_str().to_owned())
                .collect::<Vec<_>>()
        };
        assert_eq!(names(&forward), ["aaa", "zzz"]);
        assert_eq!(names(&forward), names(&backward));
        assert_eq!(
            forward, backward,
            "insertion order must not affect equality"
        );
    }

    #[test]
    fn carriers_are_found_across_loci() {
        let profile = sample();
        assert!(profile.carries(Allele::recessive(2)));
        assert!(!profile.carries(Allele::standard(99)));
    }

    #[test]
    fn homozygous_loci_are_counted() {
        assert_eq!(sample().homozygous_loci(), 1);
    }

    #[test]
    fn a_matching_schema_is_accepted() {
        let schema = [gene("coat_colour"), gene("size")];
        assert!(sample().check_against_schema(&schema).is_ok());
    }

    #[test]
    fn schema_violations_report_both_directions_at_once() {
        let schema = [gene("coat_colour"), gene("temperament")];
        let error = sample().check_against_schema(&schema).unwrap_err();
        let DomainError::GeneticSchemaMismatch {
            missing,
            unexpected,
        } = error
        else {
            panic!("wrong error variant");
        };
        assert_eq!(missing, ["temperament"]);
        assert_eq!(unexpected, ["size"]);
    }

    #[test]
    fn an_empty_profile_fails_a_non_empty_schema() {
        let error = GeneticProfile::new()
            .check_against_schema(&[gene("size")])
            .unwrap_err();
        assert!(matches!(error, DomainError::GeneticSchemaMismatch { .. }));
    }

    #[test]
    fn profiles_survive_a_serde_round_trip() {
        let profile = sample();
        let encoded = serde_json::to_string(&profile).expect("serialize");
        assert_eq!(
            serde_json::from_str::<GeneticProfile>(&encoded).expect("deserialize"),
            profile
        );
    }
}
