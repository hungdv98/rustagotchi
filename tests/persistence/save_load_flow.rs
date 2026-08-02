mod support;
use application::ports::SaveRepository;
#[test]
fn save_then_load_is_exact() {
    let temp = tempfile::tempdir().unwrap();
    let repo = persistence::SqliteSaveRepository::new(temp.path().join("save.db")).unwrap();
    let state = support::state();
    repo.save(support::slot(), &state).unwrap();
    assert_eq!(repo.load(support::slot()).unwrap(), state);
}
