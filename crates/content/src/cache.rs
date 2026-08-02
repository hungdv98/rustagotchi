//! Explicit registry cache for long-running tools and the desktop application.

use crate::{ContentError, ContentLoader, ContentRegistry};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ContentCache {
    root: PathBuf,
    current: Arc<ContentRegistry>,
}

impl ContentCache {
    pub fn load(root: impl AsRef<Path>) -> Result<Self, ContentError> {
        let root = root.as_ref().to_path_buf();
        let current = Arc::new(ContentLoader::load(&root)?);
        Ok(Self { root, current })
    }
    #[must_use]
    pub fn get(&self) -> Arc<ContentRegistry> {
        Arc::clone(&self.current)
    }
    /// Atomically replaces the registry only after the new pack fully validates.
    pub fn reload(&mut self) -> Result<Arc<ContentRegistry>, ContentError> {
        let next = Arc::new(ContentLoader::load(&self.root)?);
        self.current = Arc::clone(&next);
        Ok(next)
    }
}
