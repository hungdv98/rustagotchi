use thiserror::Error;

#[derive(Debug, Error)]
pub enum SaveError {
    #[error("save repository is locked by another process")]
    Locked,
    #[error("save slot was not found")]
    NotFound,
    #[error("save data is corrupt: {0}")]
    Corrupt(String),
    #[error("save schema {found} is unsupported; current schema is {current}")]
    UnsupportedSchema { found: u32, current: u32 },
    #[error("game state is invalid: {0}")]
    InvalidState(String),
    #[error("save storage failed: {0}")]
    Storage(String),
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Save(#[from] SaveError),
    #[error("domain operation failed: {0}")]
    Domain(String),
    #[error("content operation failed: {0}")]
    Content(String),
}

#[derive(Debug, Error)]
#[error("cannot generate a world seed: {0}")]
pub struct SeedError(pub String);
