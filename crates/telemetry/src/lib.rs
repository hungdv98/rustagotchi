//! Logging, metrics, performance sampling and crash reporting.
//!
//! `crash_report` installs a `std::panic::set_hook` handler and relies on
//! unwinding, which is why `panic = "abort"` is absent from the release profile
//! (see docs/implementation-plan.md correction C2). A crash report captures the
//! panic payload, the backtrace, the build version and the current `Tick` --
//! never save contents or player-authored text.
//!
//! `metrics` consumes `domain::events::DomainEvent` through the
//! `application::ports::Telemetry` port, so instrumentation never requires
//! touching gameplay code.
//!
//! Phase 7 builds: `logging`, `metrics`, `performance`, `crash_report`.
