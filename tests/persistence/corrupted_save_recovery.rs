mod support;
use application::ports::SaveRepository;
#[test]
fn bit_flipped_primary_is_replaced_from_backup() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("save.db");
    let repo = persistence::SqliteSaveRepository::new(&path).unwrap();
    let expected = support::state();
    repo.save(support::slot(), &expected).unwrap();
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute("UPDATE game_snapshots SET envelope=randomblob(24)", [])
        .unwrap();
    assert_eq!(repo.load(support::slot()).unwrap(), expected);
    assert_eq!(
        repo.load(support::slot()).unwrap(),
        expected,
        "restored primary must load again"
    );
}
