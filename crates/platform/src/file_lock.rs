use fs4::{FileExt, TryLockError};
use std::fs::{File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// An exclusive, non-blocking OS file lock held until this value is dropped.
#[derive(Debug)]
pub struct FileLock {
    file: File,
    path: PathBuf,
}

#[derive(Debug, Error)]
pub enum LockError {
    #[error("lock at {path} is already held")]
    AlreadyHeld { path: PathBuf },
    #[error("cannot access lock file {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

impl FileLock {
    pub fn try_acquire(path: impl AsRef<Path>) -> Result<Self, LockError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| LockError::Io {
                path: path.clone(),
                source,
            })?;
        }

        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&path)
            .map_err(|source| LockError::Io {
                path: path.clone(),
                source,
            })?;

        FileExt::try_lock(&file).map_err(|source| match source {
            TryLockError::WouldBlock => LockError::AlreadyHeld { path: path.clone() },
            TryLockError::Error(source) => LockError::Io {
                path: path.clone(),
                source,
            },
        })?;

        Ok(Self { file, path })
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        if let Err(error) = FileExt::unlock(&self.file) {
            tracing::warn!(path = %self.path.display(), %error, "failed to release file lock");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_is_exclusive_and_released_on_drop() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("app.lock");
        let first = FileLock::try_acquire(&path).unwrap();

        assert!(matches!(
            FileLock::try_acquire(&path),
            Err(LockError::AlreadyHeld { .. })
        ));

        drop(first);
        FileLock::try_acquire(path).unwrap();
    }
}
