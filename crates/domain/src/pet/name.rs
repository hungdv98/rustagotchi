use core::fmt;

use serde::{Deserialize, Serialize};

pub const MAX_PET_NAME_CHARS: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PetNameError {
    #[error("pet name must not be empty")]
    Empty,
    #[error("pet name is longer than {MAX_PET_NAME_CHARS} characters")]
    TooLong,
    #[error("pet name contains unsupported character {0:?}")]
    InvalidCharacter(char),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PetName(String);

impl PetName {
    pub fn new(value: impl AsRef<str>) -> Result<Self, PetNameError> {
        let normalized = value
            .as_ref()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if normalized.is_empty() {
            return Err(PetNameError::Empty);
        }
        if normalized.chars().count() > MAX_PET_NAME_CHARS {
            return Err(PetNameError::TooLong);
        }
        if let Some(character) = normalized.chars().find(|character| {
            !(character.is_alphanumeric() || matches!(character, ' ' | '-' | '\''))
        }) {
            return Err(PetNameError::InvalidCharacter(character));
        }
        Ok(Self(normalized))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for PetName {
    fn default() -> Self {
        Self("Companion".into())
    }
}

impl fmt::Display for PetName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_trimmed_and_allow_human_writing() {
        assert_eq!(PetName::new("  Mây   Nhỏ  ").unwrap().as_str(), "Mây Nhỏ");
        assert_eq!(PetName::new("O'Malley").unwrap().as_str(), "O'Malley");
    }

    #[test]
    fn empty_long_and_symbolic_names_are_rejected() {
        assert!(matches!(PetName::new("  "), Err(PetNameError::Empty)));
        assert!(matches!(
            PetName::new("a".repeat(MAX_PET_NAME_CHARS + 1)),
            Err(PetNameError::TooLong)
        ));
        assert!(matches!(
            PetName::new("pet!"),
            Err(PetNameError::InvalidCharacter('!'))
        ));
    }
}
