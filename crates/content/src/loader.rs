//! Deterministic, bounded content-pack loading.

use std::fs;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;

use crate::definitions::{
    CompatibilityDefinition, DefinitionList, EvolutionDefinition, IllnessDefinition,
    ItemDefinition, MoveDefinition, SpeciesDefinition, Translations,
};
use crate::error::{ContentError, SourceLocation};
use crate::formats::ron;
use crate::manifest::ContentManifest;
use crate::registry::ContentRegistry;
use crate::validator;

const MAX_CONTENT_FILE_BYTES: u64 = 1_048_576;

#[derive(Debug)]
pub(crate) struct Sourced<T> {
    pub value: T,
    pub file: PathBuf,
}

#[derive(Debug)]
pub(crate) struct RawContent {
    pub manifest: ContentManifest,
    pub species: Vec<Sourced<SpeciesDefinition>>,
    pub evolutions: Vec<Sourced<EvolutionDefinition>>,
    pub items: Vec<Sourced<ItemDefinition>>,
    pub moves: Vec<Sourced<MoveDefinition>>,
    pub illnesses: Vec<Sourced<IllnessDefinition>>,
    pub compatibility: Vec<Sourced<CompatibilityDefinition>>,
    pub translations: Translations,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ContentLoader;

impl ContentLoader {
    /// Loads `root/manifest.ron`, validates the complete pack, then builds an immutable registry.
    pub fn load(root: impl AsRef<Path>) -> Result<ContentRegistry, ContentError> {
        let root = root.as_ref();
        let manifest_path = root.join("manifest.ron");
        let manifest: ContentManifest = read_ron(&manifest_path)?;
        let manifest_issues = manifest.validate(&manifest_path);
        if !manifest_issues.is_empty() {
            return Err(ContentError::Invalid {
                issues: manifest_issues,
            });
        }

        let raw = RawContent {
            species: load_lists(root, &manifest.species)?,
            evolutions: load_lists(root, &manifest.evolutions)?,
            items: load_lists(root, &manifest.items)?,
            moves: load_lists(root, &manifest.moves)?,
            illnesses: load_lists(root, &manifest.illnesses)?,
            compatibility: load_lists(root, &manifest.compatibility)?,
            translations: read_ron(&root.join(&manifest.en_us))?,
            manifest,
        };
        let issues = validator::validate(&raw);
        if !issues.is_empty() {
            return Err(ContentError::Invalid { issues });
        }
        Ok(ContentRegistry::from_validated(raw))
    }
}

fn load_lists<T: DeserializeOwned>(
    root: &Path,
    paths: &[PathBuf],
) -> Result<Vec<Sourced<T>>, ContentError> {
    let mut loaded = Vec::new();
    for relative in paths {
        let path = root.join(relative);
        let list: DefinitionList<T> = read_ron(&path)?;
        loaded.extend(list.definitions.into_iter().map(|value| Sourced {
            value,
            file: path.clone(),
        }));
    }
    Ok(loaded)
}

fn read_ron<T: DeserializeOwned>(path: &Path) -> Result<T, ContentError> {
    let metadata = fs::metadata(path).map_err(|source| ContentError::Io {
        location: SourceLocation::file(path),
        source,
    })?;
    if metadata.len() > MAX_CONTENT_FILE_BYTES {
        return Err(ContentError::Malformed {
            location: SourceLocation::file(path),
            message: format!(
                "file is {} bytes; maximum is {MAX_CONTENT_FILE_BYTES}",
                metadata.len()
            ),
        });
    }
    let source = fs::read_to_string(path).map_err(|source| ContentError::Io {
        location: SourceLocation::file(path),
        source,
    })?;
    ron::parse(path, &source)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn parse_errors_include_line_and_column() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(
            temp.path().join("manifest.ron"),
            "(\n format_version: nope,\n)",
        )
        .unwrap();
        let error = ContentLoader::load(temp.path()).unwrap_err().to_string();
        assert!(error.contains("manifest.ron:2:"), "{error}");
    }
}
