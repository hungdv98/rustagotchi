use std::fs;
use std::path::{Path, PathBuf};

use content::{ContentError, ContentLoader, ValidationRule};

fn assets() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

fn fixture() -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    copy_tree(&assets(), temp.path());
    temp
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn replace(temp: &tempfile::TempDir, relative: &str, from: &str, to: &str) {
    let path = temp.path().join(relative);
    let source = fs::read_to_string(&path).unwrap();
    assert!(source.contains(from), "fixture did not contain {from:?}");
    fs::write(path, source.replacen(from, to, 1)).unwrap();
}

fn assert_rule(temp: &tempfile::TempDir, expected: ValidationRule) {
    let error = ContentLoader::load(temp.path()).unwrap_err();
    let ContentError::Invalid { issues } = error else {
        panic!("expected validation error, got {error}");
    };
    assert!(
        issues.iter().any(|issue| issue.rule == expected),
        "expected {} in {issues:#?}",
        expected.code()
    );
}

#[test]
fn production_pack_is_valid_and_indexed() {
    let registry = ContentLoader::load(assets()).unwrap();
    assert_eq!(registry.counts().species, 3);
    assert_eq!(registry.counts().items, 20);
    assert_eq!(registry.counts().moves, 16);
    let id = domain::identifiers::SpeciesId::new("emberling").unwrap();
    assert_eq!(
        registry.get_species(&id).unwrap().name_key,
        "species.emberling"
    );
}

#[test]
fn rejects_duplicate_ids_across_definition_kinds() {
    let temp = fixture();
    replace(
        &temp,
        "content/items.ron",
        "id: \"apple_slice\"",
        "id: \"emberling\"",
    );
    assert_rule(&temp, ValidationRule::DuplicateId);
}

#[test]
fn rejects_dangling_references() {
    let temp = fixture();
    replace(
        &temp,
        "content/species.ron",
        "\"tackle\", \"warm_nuzzle\"",
        "\"missing_move\", \"warm_nuzzle\"",
    );
    assert_rule(&temp, ValidationRule::DanglingReference);
}

#[test]
fn rejects_empty_stage_move_pools() {
    let temp = fixture();
    replace(
        &temp,
        "content/species.ron",
        "elder: [\"flame_arc\", \"solar_roar\"]",
        "elder: []",
    );
    assert_rule(&temp, ValidationRule::EmptyMovePool);
}

#[test]
fn rejects_evolution_cycles() {
    let temp = fixture();
    replace(
        &temp,
        "content/evolutions.ron",
        "    (from: \"stormling\", to: \"aquafin\", minimum_level: 40),",
        "    (from: \"stormling\", to: \"aquafin\", minimum_level: 40),\n    (from: \"aquafin\", to: \"emberling\", minimum_level: 60),",
    );
    assert_rule(&temp, ValidationRule::EvolutionCycle);
}

#[test]
fn rejects_unreachable_species() {
    let temp = fixture();
    replace(&temp, "content/species.ron", "base: true", "base: false");
    assert_rule(&temp, ValidationRule::UnreachableSpecies);
}

#[test]
fn rejects_asymmetric_compatibility() {
    let temp = fixture();
    replace(
        &temp,
        "content/compatibility.ron",
        "second: \"stormling\", chance_percent: \"55\"",
        "second: \"stormling\", chance_percent: \"54\"",
    );
    assert_rule(&temp, ValidationRule::AsymmetricCompatibility);
}

#[test]
fn rejects_incomplete_compatibility() {
    let temp = fixture();
    replace(
        &temp,
        "content/compatibility.ron",
        "    (first: \"aquafin\", second: \"aquafin\", chance_percent: \"70\"),\n",
        "",
    );
    assert_rule(&temp, ValidationRule::IncompleteCompatibility);
}

#[test]
fn rejects_out_of_range_values() {
    let temp = fixture();
    replace(&temp, "content/moves.ron", "power: 30", "power: 999");
    assert_rule(&temp, ValidationRule::ValueOutOfRange);
}

#[test]
fn rejects_missing_translations() {
    let temp = fixture();
    replace(
        &temp,
        "content/species.ron",
        "species.emberling",
        "species.missing",
    );
    assert_rule(&temp, ValidationRule::MissingTranslation);
}
