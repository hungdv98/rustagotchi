//! Content-pack manifest and its safety checks.

use crate::error::{ContentIssue, SourceLocation, ValidationRule};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

pub const SUPPORTED_FORMAT_VERSION: u16 = 1;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentManifest {
    pub format_version: u16,
    pub pack_id: String,
    #[serde(default)]
    pub species: Vec<PathBuf>,
    #[serde(default)]
    pub evolutions: Vec<PathBuf>,
    #[serde(default)]
    pub items: Vec<PathBuf>,
    #[serde(default)]
    pub moves: Vec<PathBuf>,
    #[serde(default)]
    pub illnesses: Vec<PathBuf>,
    #[serde(default)]
    pub compatibility: Vec<PathBuf>,
    pub en_us: PathBuf,
}

impl ContentManifest {
    pub(crate) fn validate(&self, manifest_path: &Path) -> Vec<ContentIssue> {
        let location = SourceLocation::file(manifest_path);
        let mut issues = Vec::new();
        if self.format_version != SUPPORTED_FORMAT_VERSION {
            issues.push(issue(
                &location,
                format!(
                    "unsupported format version {}; expected {SUPPORTED_FORMAT_VERSION}",
                    self.format_version
                ),
            ));
        }
        if domain::identifiers::ContentKey::new("pack id", &self.pack_id).is_err() {
            issues.push(issue(
                &location,
                format!("pack id {:?} must be snake_case", self.pack_id),
            ));
        }
        let mut seen = BTreeSet::new();
        for path in self.all_paths() {
            if !is_safe_relative(path) {
                issues.push(issue(
                    &location,
                    format!(
                        "content path {:?} must be relative and contain no `..`",
                        path.display()
                    ),
                ));
            } else if !seen.insert(path.to_path_buf()) {
                issues.push(issue(
                    &location,
                    format!("content path {:?} is listed more than once", path.display()),
                ));
            }
        }
        issues
    }

    fn all_paths(&self) -> impl Iterator<Item = &Path> {
        self.species
            .iter()
            .chain(&self.evolutions)
            .chain(&self.items)
            .chain(&self.moves)
            .chain(&self.illnesses)
            .chain(&self.compatibility)
            .chain(std::iter::once(&self.en_us))
            .map(PathBuf::as_path)
    }
}

fn is_safe_relative(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}

fn issue(location: &SourceLocation, message: String) -> ContentIssue {
    ContentIssue {
        rule: ValidationRule::InvalidManifest,
        location: location.clone(),
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn refuses_paths_that_escape_the_pack() {
        let manifest = ContentManifest {
            format_version: 1,
            pack_id: "base".into(),
            species: vec!["../secret.ron".into()],
            evolutions: vec![],
            items: vec![],
            moves: vec![],
            illnesses: vec![],
            compatibility: vec![],
            en_us: "i18n/en_us.ron".into(),
        };
        assert!(!manifest.validate(Path::new("manifest.ron")).is_empty());
    }
}
