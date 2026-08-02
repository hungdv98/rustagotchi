mod support;
use application::ports::SaveRepository;
#[test]
fn rejected_write_leaves_previous_transaction_intact() {
    let temp = tempfile::tempdir().unwrap();
    let repo = persistence::SqliteSaveRepository::new(temp.path().join("save.db")).unwrap();
    let original = support::state();
    repo.save(support::slot(), &original).unwrap();
    let mut invalid = original.clone();
    invalid.simulation.tick.0 += 1;
    assert!(repo.save(support::slot(), &invalid).is_err());
    assert_eq!(repo.load(support::slot()).unwrap(), original);
}
