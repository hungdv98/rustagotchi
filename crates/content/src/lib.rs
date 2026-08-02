//! Data-driven content: loading, validation and interning of content packs.
//!
//! The `definitions` module mirrors the RON on disk using *designer* units
//! (decimals, human-readable ids). Those types are deliberately not domain
//! types. Conversion to `domain` happens exactly once, at load time, and is the
//! only place where authored decimals become `domain::units::Fixed` -- which is
//! what keeps the integer-determinism rule of ADR 0006 enforceable.
//!
//! Load order: `loader` (parse) -> `validator` (reference integrity, graph
//! reachability, range checks) -> `registry` (immutable, id-interned, infallible
//! lookup). After validation succeeds, no content lookup can fail at runtime.
//!
//! Phase 2 builds: `manifest`, `loader`, `registry`, `validator`, `cache`,
//! `definitions/*`, `formats/ron`. Phase 7 adds `localization/*`.

pub mod cache;
pub mod decimal;
pub mod definitions;
pub mod error;
pub mod formats;
pub mod loader;
pub mod manifest;
pub mod registry;
mod validator;

pub use error::{ContentError, ContentIssue, SourceLocation, ValidationRule};
pub use loader::ContentLoader;
pub use registry::{ContentCounts, ContentRegistry};
