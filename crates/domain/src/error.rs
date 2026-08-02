//! The domain's error type.
//!
//! Domain operations never panic on bad *data* — they return one of these. The
//! panics that do exist ([`crate::units::Milli::scale`] with a zero denominator,
//! and similar) are reserved for programming errors that no input can trigger.
//! That split is what lets the fuzz targets treat "did not panic" as a
//! meaningful assertion.

use crate::identifiers::PetId;
use crate::validation::ValidationError;

/// Anything the domain can refuse to do.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum DomainError {
    #[error("invalid value: {0}")]
    Validation(#[from] ValidationError),

    #[error("no {entity} with id {id}")]
    NotFound { entity: &'static str, id: String },

    #[error("cannot {action}: {reason}")]
    NotAllowed {
        action: &'static str,
        reason: &'static str,
    },

    #[error("pet {pet} cannot {action} while it is dead")]
    PetIsDead { pet: PetId, action: &'static str },

    #[error("a life stage cannot move backwards ({from} -> {to})")]
    LifeStageRegression {
        from: &'static str,
        to: &'static str,
    },

    #[error(
        "genetic profile does not match its species: missing {missing:?}, unexpected {unexpected:?}"
    )]
    GeneticSchemaMismatch {
        missing: Vec<String>,
        unexpected: Vec<String>,
    },

    #[error("not enough {resource}: needed {needed}, had {available}")]
    Insufficient {
        resource: &'static str,
        needed: i64,
        available: i64,
    },

    #[error("{container} is full ({capacity} slots)")]
    AtCapacity {
        container: &'static str,
        capacity: usize,
    },
}

impl DomainError {
    /// Convenience for the common "looked it up, was not there" case.
    #[must_use]
    pub fn not_found(entity: &'static str, id: impl core::fmt::Display) -> Self {
        DomainError::NotFound {
            entity,
            id: id.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_errors_convert_with_the_question_mark_operator() {
        fn fallible() -> Result<(), DomainError> {
            crate::validation::require_range("hp", 99, 0, 10)?;
            Ok(())
        }
        assert!(matches!(fallible(), Err(DomainError::Validation(_))));
    }

    #[test]
    fn messages_name_what_failed() {
        let error = DomainError::not_found("species", "emberling");
        assert_eq!(error.to_string(), "no species with id emberling");
    }
}
