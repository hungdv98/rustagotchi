mod support;
use application::ports::SaveRepository;
use std::path::Path;
#[test]
fn schema_v1_fixture_loads_under_current_schema() {
    assert_eq!(
        support::state().metadata.schema_version,
        application::GameState::SCHEMA_VERSION
    );
    let temp = tempfile::tempdir().unwrap();
    let repo = persistence::SqliteSaveRepository::new(temp.path().join("schema_v1.db")).unwrap();
    let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/saves/schema_v1.json");
    let legacy: application::GameState =
        serde_json::from_str(&std::fs::read_to_string(fixture_path).unwrap()).unwrap();
    assert_eq!(legacy.metadata.schema_version, 1);
    let fixture = legacy.migrate_from(1).unwrap();
    repo.save(support::slot(), &fixture).unwrap();
    let loaded = repo.load(support::slot()).unwrap();
    assert_eq!(
        loaded.metadata.schema_version,
        application::GameState::SCHEMA_VERSION
    );
    assert_eq!(loaded, fixture);
}

#[test]
fn every_committed_schema_fixture_decodes() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/saves");
    let mut versions = Vec::new();
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|value| value == "json") {
            let state: application::GameState =
                serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            let schema = state.metadata.schema_version;
            state.migrate_from(schema).unwrap().validate().unwrap();
            versions.push(schema);
        }
    }
    versions.sort_unstable();
    assert_eq!(
        versions,
        (1..=application::GameState::SCHEMA_VERSION).collect::<Vec<_>>(),
        "one committed fixture is required per schema version"
    );
}
