//! Durable storage. Implements `application::ports::SaveRepository`.
//!
//! The versioned snapshot is authoritative; the normalized tables are a derived
//! read-model, dropped and rebuilt from the snapshot inside the same
//! transaction (ADR 0008). Nothing incrementally patches the tables, so they
//! cannot drift out of agreement with the snapshot -- which removes the largest
//! single source of save corruption in a design of this shape.
//!
//! Payload: `zstd(msgpack(GameState))` plus a checksum over the compressed
//! bytes, wrapped in `serialization::save_envelope::SaveEnvelope`.
//!
//! Migrations are Rust functions over snapshot types
//! (`fn migrate_v3_to_v4(SnapshotV3) -> SnapshotV4`), not hand-written SQL. The
//! `.sql` files under `migrations/` define the *derived* schema only; changing
//! them triggers a rebuild, never a data migration.
//!
//! Every `schema_version` gets a committed fixture under
//! `tests/fixtures/saves/`, forever. `tests/persistence/migration_compatibility.rs`
//! iterates that directory so a missing migration cannot pass CI.
//!
//! Phase 4 builds: `connection`, `transaction`, `migration`, `backup`,
//! `recovery`, `validation`, `repositories/*`, `models/*`, `serialization/*`.

mod envelope;
mod sqlite;

pub use envelope::{CURRENT_FORMAT_VERSION, SaveEnvelope};
pub use sqlite::SqliteSaveRepository;
