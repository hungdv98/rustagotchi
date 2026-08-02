use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum PathError {
    #[error("the operating system has no application data directory")]
    Unavailable,
    #[error("cannot create application directory: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    pub data: PathBuf,
    pub saves: PathBuf,
    pub logs: PathBuf,
    pub config: PathBuf,
}

impl AppPaths {
    pub fn discover() -> Result<Self, PathError> {
        let project = directories::ProjectDirs::from("com", "Rustagotchi", "Rustagotchi")
            .ok_or(PathError::Unavailable)?;
        Ok(Self::from_root(project.data_local_dir()))
    }

    #[must_use]
    pub fn from_root(root: impl AsRef<Path>) -> Self {
        let data = root.as_ref().to_path_buf();
        Self {
            saves: data.join("saves"),
            logs: data.join("logs"),
            config: data.join("config"),
            data,
        }
    }

    pub fn ensure(&self) -> Result<(), PathError> {
        for path in [&self.data, &self.saves, &self.logs, &self.config] {
            std::fs::create_dir_all(path)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_creates_the_complete_layout() {
        let temporary = tempfile::tempdir().unwrap();
        let paths = AppPaths::from_root(temporary.path().join("app"));
        paths.ensure().unwrap();
        assert!(paths.saves.is_dir());
        assert!(paths.logs.is_dir());
        assert!(paths.config.is_dir());
    }
}
