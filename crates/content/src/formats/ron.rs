//! RON parsing with source positions preserved for diagnostics.

use crate::error::{ContentError, SourceLocation};
use serde::de::DeserializeOwned;
use std::path::Path;

pub(crate) fn parse<T: DeserializeOwned>(path: &Path, source: &str) -> Result<T, ContentError> {
    ron::from_str(source).map_err(|error| {
        let position = error.span.start;
        ContentError::Parse {
            location: SourceLocation::at(path, position.line as u32, position.col as u32),
            message: error.code.to_string(),
        }
    })
}
