# ADR 0008 — The snapshot is authoritative; SQL tables are a derived read-model

- **Status:** Accepted
- **Date:** 2026-08-02
- **Amends:** the persistence model in `docs/init-plan.md`

## Context

`docs/init-plan.md` proposes writing both normalized tables (`pets`,
`pet_genetics`, `lineage`, …) *and* a versioned snapshot, both treated as real
state. Two authoritative representations of the same data, incrementally updated,
is the most reliable way to produce save corruption: every new field is a chance
to update one and forget the other, and the resulting inconsistency is silent
until a player loses a pet.

The normalized tables do earn their place, though — the save-slot screen, the
lineage view and battle history all want queries rather than a full deserialize.

## Decision

Split authority from convenience.

- **Authoritative:** one versioned snapshot per slot —
  `zstd(msgpack(GameState))` plus a checksum, wrapped in a `SaveEnvelope` that
  carries `format_version`, `schema_version` and the producing build version.
- **Derived:** the normalized tables. Inside the same transaction they are
  **dropped and rebuilt** from the snapshot. Nothing patches them incrementally,
  so they cannot disagree with the snapshot.

Save transaction:

```text
acquire advisory lock -> validate GameState -> serialize+compress+checksum
-> BEGIN IMMEDIATE -> write snapshot -> rebuild derived tables
-> write slot metadata -> COMMIT -> rotate backups -> release lock
```

Load: read snapshot → verify checksum → migrate if `schema_version` is behind →
rebuild derived tables → hand `GameState` to the application.

Migrations are **Rust functions over snapshot types**
(`fn migrate_v3_to_v4(SnapshotV3) -> SnapshotV4`), one per version step. The
`.sql` files under `crates/persistence/migrations/` define the derived schema
only; changing one triggers a rebuild, never a data migration.

Every `schema_version` gets a committed fixture under `tests/fixtures/saves/`,
permanently. `tests/persistence/migration_compatibility.rs` iterates that
directory, so a missing migration cannot pass CI.

## Consequences

- Adding a field to `GameState` requires no SQL and cannot desynchronise the two
  representations.
- Schema changes to the read-model are free: change the DDL, rebuild.
- Corruption is detectable (checksum) and recoverable (rotating backups), because
  there is exactly one thing to check and restore.
- Cost: a full rebuild on every save. At the target save size this is
  milliseconds; if a future save outgrows it, the fix is to rebuild lazily on
  first query, not to reintroduce incremental writes.
- The whole state is loaded to answer any deep query. Acceptable — a save is one
  player's pets, not a database.

## Alternatives considered

- **Both authoritative, incrementally updated** (as in `init-plan.md`). Rejected:
  dual-write inconsistency, and hand-written SQL migrations for every field.
- **Tables only, no snapshot.** Rejected: every gameplay change becomes a schema
  migration, and version-to-version migration testing gets much harder.
- **Snapshot only, no tables.** Viable, and worth revisiting if the read-model
  turns out to be underused; rejected for now because the slot-list and lineage
  screens genuinely want queries.
