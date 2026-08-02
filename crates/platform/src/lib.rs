//! Concrete implementations of `application::ports` against the host OS.
//!
//! This crate is the only place allowed to read the wall clock, touch the
//! filesystem outside of `persistence`, or call OS notification APIs. Each
//! module is one port implementation, so the core can be driven entirely by
//! fakes from `rustagotchi-test-support` with no cfg-gating.
//!
//! `random` generates *seeds* only. Once a world seed exists, all randomness is
//! derived deterministically inside `domain::rng` (ADR 0007).
//!
//! Phase 6 builds: `paths`, `clock`, `random`, `notifications`, `file_lock`,
//! `process_lock`, `lifecycle`.

pub mod clock;
pub mod file_lock;
pub mod lifecycle;
pub mod notifications;
pub mod paths;
pub mod process_lock;
pub mod random;

pub use clock::SystemClock;
pub use file_lock::{FileLock, LockError};
pub use lifecycle::{Lifecycle, LifecycleError, LifecycleState};
pub use notifications::DesktopNotifications;
pub use paths::{AppPaths, PathError};
pub use process_lock::ProcessLock;
pub use random::SystemRandomSource;
