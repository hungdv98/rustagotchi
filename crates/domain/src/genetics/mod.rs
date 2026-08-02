//! Genetics: what a pet carries, what it shows, and where it came from.
//!
//! This module is data and rules only. The engines that *use* it — gene
//! inheritance, mutation rolls, species resolution — live in
//! `simulation::breeding` and arrive in Phase 5. Keeping the split means the
//! genetic model can be exercised exhaustively without a simulation running.
//!
//! One rule runs through all of it: **nothing may depend on which parent is
//! called maternal.** That is an accident of how a breeding call was written,
//! and if expression or ordering depended on it, the same cross would produce
//! different results depending on argument order.

pub mod allele;
pub mod gene;
pub mod lineage;
pub mod mutation;
pub mod phenotype;
pub mod profile;

pub use allele::{Allele, AlleleSymbol, Dominance};
pub use gene::{Expression, GenePair};
pub use lineage::{Ancestry, Parentage};
pub use mutation::{Mutation, MutationRate};
pub use phenotype::Phenotype;
pub use profile::GeneticProfile;
