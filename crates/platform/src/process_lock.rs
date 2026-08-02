use crate::{AppPaths, FileLock, LockError};
use std::path::Path;

/// Prevents two game processes from mutating the same local data directory.
#[derive(Debug)]
pub struct ProcessLock(FileLock);

impl ProcessLock {
    pub fn acquire(paths: &AppPaths) -> Result<Self, LockError> {
        FileLock::try_acquire(paths.data.join("rustagotchi.lock")).map(Self)
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        self.0.path()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_one_process_lock_can_be_held() {
        let directory = tempfile::tempdir().unwrap();
        let paths = AppPaths::from_root(directory.path());
        let first = ProcessLock::acquire(&paths).unwrap();

        assert!(matches!(
            ProcessLock::acquire(&paths),
            Err(LockError::AlreadyHeld { .. })
        ));
        assert!(first.path().ends_with("rustagotchi.lock"));
    }
}
