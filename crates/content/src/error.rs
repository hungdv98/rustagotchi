//! Content loading and validation failures.
//!
//! Errors here are read by designers, not programmers. Every one names the file
//! it came from, and parse failures carry the line and column RON reports, so
//! the feedback loop is "fix line 12" rather than "something is wrong
//! somewhere".

use std::path::{Path, PathBuf};

use thiserror::Error;

/// Where in the content tree a problem is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
    pub file: PathBuf,
    /// Present for parse errors, which RON reports with a position. Semantic
    /// errors know the file and the offending id but not a line.
    pub position: Option<(u32, u32)>,
}

impl SourceLocation {
    #[must_use]
    pub fn file(path: impl AsRef<Path>) -> Self {
        SourceLocation {
            file: path.as_ref().to_path_buf(),
            position: None,
        }
    }

    #[must_use]
    pub fn at(path: impl AsRef<Path>, line: u32, column: u32) -> Self {
        SourceLocation {
            file: path.as_ref().to_path_buf(),
            position: Some((line, column)),
        }
    }
}

impl std::fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.position {
            Some((line, column)) => write!(f, "{}:{}:{}", self.file.display(), line, column),
            None => write!(f, "{}", self.file.display()),
        }
    }
}

/// A failure that stops loading entirely.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ContentError {
    #[error("{location}: {source}")]
    Io {
        location: SourceLocation,
        #[source]
        source: std::io::Error,
    },

    #[error("{location}: {message}")]
    Parse {
        location: SourceLocation,
        message: String,
    },

    #[error("{location}: {message}")]
    Malformed {
        location: SourceLocation,
        message: String,
    },

    #[error("content pack failed validation with {} problem(s):\n{}", issues.len(), format_issues(issues))]
    Invalid { issues: Vec<ContentIssue> },
}

/// One thing wrong with an otherwise well-formed content pack.
///
/// Validation collects every issue rather than stopping at the first. A designer
/// fixing eight dangling references wants all eight now, not eight round trips.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentIssue {
    pub rule: ValidationRule,
    pub location: SourceLocation,
    pub message: String,
}

impl std::fmt::Display for ContentIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}] {}: {}",
            self.rule.code(),
            self.location,
            self.message
        )
    }
}

/// Which validation rule was broken.
///
/// A stable code per rule, so a designer can search for `E003` and CI output can
/// be grouped by rule rather than by message text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum ValidationRule {
    InvalidManifest,
    DuplicateId,
    DanglingReference,
    EmptyMovePool,
    EvolutionCycle,
    UnreachableSpecies,
    AsymmetricCompatibility,
    IncompleteCompatibility,
    ValueOutOfRange,
    MissingTranslation,
}

impl ValidationRule {
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            ValidationRule::InvalidManifest => "E000",
            ValidationRule::DuplicateId => "E001",
            ValidationRule::DanglingReference => "E002",
            ValidationRule::EmptyMovePool => "E003",
            ValidationRule::EvolutionCycle => "E004",
            ValidationRule::UnreachableSpecies => "E005",
            ValidationRule::AsymmetricCompatibility => "E006",
            ValidationRule::IncompleteCompatibility => "E007",
            ValidationRule::ValueOutOfRange => "E008",
            ValidationRule::MissingTranslation => "E009",
        }
    }

    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
            ValidationRule::InvalidManifest => "the manifest is unsafe or unsupported",
            ValidationRule::DuplicateId => "two definitions share an id",
            ValidationRule::DanglingReference => {
                "a reference points at something that does not exist"
            }
            ValidationRule::EmptyMovePool => "a species has no usable move at some stage",
            ValidationRule::EvolutionCycle => "the evolution graph contains a cycle",
            ValidationRule::UnreachableSpecies => {
                "a species cannot be reached from any base species"
            }
            ValidationRule::AsymmetricCompatibility => {
                "breeding compatibility disagrees with itself"
            }
            ValidationRule::IncompleteCompatibility => "a species pair has no compatibility entry",
            ValidationRule::ValueOutOfRange => "a numeric field is outside its declared bounds",
            ValidationRule::MissingTranslation => "a definition has no en-US translation",
        }
    }
}

fn format_issues(issues: &[ContentIssue]) -> String {
    issues
        .iter()
        .map(|issue| format!("  {issue}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locations_render_with_and_without_a_position() {
        assert_eq!(SourceLocation::file("a/b.ron").to_string(), "a/b.ron");
        assert_eq!(
            SourceLocation::at("a/b.ron", 12, 4).to_string(),
            "a/b.ron:12:4"
        );
    }

    #[test]
    fn rule_codes_are_unique() {
        let rules = [
            ValidationRule::InvalidManifest,
            ValidationRule::DuplicateId,
            ValidationRule::DanglingReference,
            ValidationRule::EmptyMovePool,
            ValidationRule::EvolutionCycle,
            ValidationRule::UnreachableSpecies,
            ValidationRule::AsymmetricCompatibility,
            ValidationRule::IncompleteCompatibility,
            ValidationRule::ValueOutOfRange,
            ValidationRule::MissingTranslation,
        ];
        let mut codes: Vec<&str> = rules.iter().map(|rule| rule.code()).collect();
        codes.sort_unstable();
        let count = codes.len();
        codes.dedup();
        assert_eq!(codes.len(), count, "rule codes must not collide");
    }

    #[test]
    fn an_invalid_pack_lists_every_problem() {
        let error = ContentError::Invalid {
            issues: vec![
                ContentIssue {
                    rule: ValidationRule::DanglingReference,
                    location: SourceLocation::file("species/a.ron"),
                    message: "move `nope` does not exist".into(),
                },
                ContentIssue {
                    rule: ValidationRule::DuplicateId,
                    location: SourceLocation::file("items/b.ron"),
                    message: "item `berry` is defined twice".into(),
                },
            ],
        };
        let rendered = error.to_string();
        assert!(rendered.contains("2 problem(s)"));
        assert!(rendered.contains("E002"));
        assert!(rendered.contains("E001"));
        assert!(rendered.contains("species/a.ron"));
    }
}
