//! Input validation shared across the domain.
//!
//! Validation happens at construction: a value that exists is a value that is
//! valid. That is why the id types below cannot be built from an arbitrary
//! string, and why content lookup is infallible after Phase 2 validation — there
//! is no `Option` left to unwrap at the point of use.

use serde::{Deserialize, Serialize};

/// Why a value was rejected.
///
/// `field` is a `&'static str` naming the thing that failed, so error messages
/// are useful without allocating on the happy path.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ValidationError {
    #[error("{field} must not be empty")]
    Empty { field: &'static str },

    #[error("{field} is {length} characters long; the maximum is {maximum}")]
    TooLong {
        field: &'static str,
        length: usize,
        maximum: usize,
    },

    #[error("{field} contains {found:?}; only lowercase ASCII letters, digits and '_' are allowed")]
    InvalidCharacter { field: &'static str, found: char },

    #[error("{field} must start with a lowercase ASCII letter")]
    BadFirstCharacter { field: &'static str },

    #[error("{field} must be within {minimum}..={maximum}, but was {value}")]
    OutOfRange {
        field: &'static str,
        value: i64,
        minimum: i64,
        maximum: i64,
    },
}

/// Rejects `value` unless it lies within `minimum..=maximum`.
pub fn require_range(
    field: &'static str,
    value: i64,
    minimum: i64,
    maximum: i64,
) -> Result<(), ValidationError> {
    if (minimum..=maximum).contains(&value) {
        Ok(())
    } else {
        Err(ValidationError::OutOfRange {
            field,
            value,
            minimum,
            maximum,
        })
    }
}

/// Checks the `snake_case` identifier grammar used by every content key.
///
/// The grammar is deliberately narrow — lowercase ASCII, digits and underscore,
/// starting with a letter. Content ids end up in filenames, localization keys
/// and save data, so allowing Unicode or mixed case would mean three separate
/// normalisation bugs waiting to happen.
pub fn require_identifier(
    field: &'static str,
    value: &str,
    maximum: usize,
) -> Result<(), ValidationError> {
    if value.is_empty() {
        return Err(ValidationError::Empty { field });
    }
    if value.len() > maximum {
        return Err(ValidationError::TooLong {
            field,
            length: value.len(),
            maximum,
        });
    }
    let first = value.chars().next().expect("checked non-empty above");
    if !first.is_ascii_lowercase() {
        return Err(ValidationError::BadFirstCharacter { field });
    }
    for character in value.chars() {
        let allowed =
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_';
        if !allowed {
            return Err(ValidationError::InvalidCharacter {
                field,
                found: character,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_are_inclusive() {
        assert!(require_range("hp", 0, 0, 10).is_ok());
        assert!(require_range("hp", 10, 0, 10).is_ok());
        assert!(require_range("hp", -1, 0, 10).is_err());
        assert!(require_range("hp", 11, 0, 10).is_err());
    }

    #[test]
    fn accepts_snake_case_identifiers() {
        assert!(require_identifier("id", "emberling", 64).is_ok());
        assert!(require_identifier("id", "ember_ling_2", 64).is_ok());
    }

    #[test]
    fn rejects_everything_outside_the_grammar() {
        assert!(require_identifier("id", "", 64).is_err());
        assert!(require_identifier("id", "Emberling", 64).is_err());
        assert!(require_identifier("id", "2emberling", 64).is_err());
        assert!(require_identifier("id", "_emberling", 64).is_err());
        assert!(require_identifier("id", "ember-ling", 64).is_err());
        assert!(require_identifier("id", "ember ling", 64).is_err());
        assert!(require_identifier("id", "emberlïng", 64).is_err());
        assert!(require_identifier("id", &"a".repeat(65), 64).is_err());
    }

    #[test]
    fn reports_the_offending_character() {
        let error = require_identifier("id", "ember-ling", 64).unwrap_err();
        assert_eq!(
            error,
            ValidationError::InvalidCharacter {
                field: "id",
                found: '-'
            }
        );
    }
}
