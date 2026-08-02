//! Shared scaffolding for the cross-crate suites.
//!
//! The determinism suites are written *before* the simulation they check, on
//! purpose. Determinism is close to impossible to retrofit onto code that has
//! already drifted, so the contract is stated first and the implementation is
//! made to satisfy it — not the other way round.

pub mod harness;
