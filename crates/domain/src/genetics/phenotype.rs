//! What a genetic profile actually shows.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::gene::Expression;
use super::profile::GeneticProfile;
use crate::identifiers::GeneId;

/// The expressed form of every locus.
///
/// Derived, never stored in a save: it is a pure function of the profile, so
/// persisting it would create a second source of truth that can fall out of step
/// — the same mistake ADR 0008 avoids at the save level.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Phenotype {
    expressed: BTreeMap<GeneId, Expression>,
}

impl Phenotype {
    #[must_use]
    pub fn of(profile: &GeneticProfile) -> Self {
        Phenotype {
            expressed: profile
                .iter()
                .map(|(gene, pair)| (gene.clone(), pair.expressed()))
                .collect(),
        }
    }

    #[must_use]
    pub fn at(&self, gene: &GeneId) -> Option<&Expression> {
        self.expressed.get(gene)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&GeneId, &Expression)> {
        self.expressed.iter()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.expressed.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.expressed.is_empty()
    }

    #[must_use]
    pub fn codominant_loci(&self) -> usize {
        self.expressed
            .values()
            .filter(|expression| expression.is_codominant())
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genetics::allele::Allele;
    use crate::genetics::gene::GenePair;

    fn gene(name: &str) -> GeneId {
        GeneId::new(name).expect("valid gene id")
    }

    #[test]
    fn every_locus_is_expressed() {
        let profile = GeneticProfile::from_loci([
            (
                gene("coat_colour"),
                GenePair::new(Allele::dominant(1), Allele::recessive(2)),
            ),
            (
                gene("size"),
                GenePair::new(Allele::standard(3), Allele::standard(4)),
            ),
        ]);
        let phenotype = Phenotype::of(&profile);

        assert_eq!(phenotype.len(), profile.len());
        assert_eq!(
            phenotype.at(&gene("coat_colour")),
            Some(&Expression::Dominant(Allele::dominant(1)))
        );
        assert_eq!(phenotype.codominant_loci(), 1);
    }

    #[test]
    fn the_phenotype_is_a_pure_function_of_the_profile() {
        let profile = GeneticProfile::from_loci([(
            gene("size"),
            GenePair::new(Allele::standard(3), Allele::standard(4)),
        )]);
        assert_eq!(Phenotype::of(&profile), Phenotype::of(&profile));
    }

    #[test]
    fn parent_order_does_not_change_the_phenotype() {
        let forward = GeneticProfile::from_loci([(
            gene("size"),
            GenePair::new(Allele::standard(3), Allele::standard(4)),
        )]);
        let reversed = GeneticProfile::from_loci([(
            gene("size"),
            GenePair::new(Allele::standard(4), Allele::standard(3)),
        )]);
        assert_eq!(Phenotype::of(&forward), Phenotype::of(&reversed));
    }

    #[test]
    fn an_empty_profile_expresses_nothing() {
        assert!(Phenotype::of(&GeneticProfile::new()).is_empty());
    }
}
