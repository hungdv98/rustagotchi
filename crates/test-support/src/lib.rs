//! Shared test scaffolding. Never a dependency of a shipping binary.
//!
//! The testing strategy in docs/implementation-plan.md is only affordable if
//! writing a test is cheap, which is what this crate is for. Extend it rather
//! than re-deriving scaffolding in each suite.
//!
//! ```
//! use test_support::builders::PetBuilder;
//!
//! let pet = PetBuilder::new().adult().starving().build();
//! assert!(pet.needs.hunger.is_empty());
//! assert_eq!(pet.stage, domain::pet::LifeStage::Adult);
//! ```
//!
//! Landed: [`assertions`], [`builders`], [`fake_clock`], [`seeded_rng`].
//!
//! Still to come: `fixtures` (an in-memory `ContentRegistry`, Phase 2) and
//! `in_memory_repository` (a `SaveRepository` fake, Phase 4). Both need types
//! that do not exist yet.

pub mod assertions;
pub mod builders;
pub mod fake_clock;
pub mod seeded_rng;

pub use assertions::{assert_emitted, assert_not_emitted, assert_state_eq};
pub use builders::{NeedsBuilder, PetBuilder, test_pet_id};
pub use fake_clock::FakeClock;
pub use seeded_rng::{TEST_SEED, test_rng, test_rng_at};
