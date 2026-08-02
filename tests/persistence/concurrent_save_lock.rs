mod support;
use application::ports::SaveRepository;
use fs4::FileExt;
#[test]
fn second_writer_fails_fast() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("save.db");
    let repo = persistence::SqliteSaveRepository::new(&path).unwrap();
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path.with_extension("lock"))
        .unwrap();
    FileExt::lock(&lock).unwrap();
    assert!(matches!(
        repo.save(support::slot(), &support::state()),
        Err(application::SaveError::Locked)
    ));
    FileExt::unlock(&lock).unwrap();
}
