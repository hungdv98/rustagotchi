# ADR 0004 — SQLite as the save container

- **Status:** Accepted
- **Date:** 2026-08-02

## Context

Saves must survive power loss mid-write, support multiple slots, allow the UI to
list and query slots without loading a full game, and be recoverable when
corrupted. A plain file per slot gets atomicity via rename but nothing else.

## Decision

One SQLite database (`rusqlite`, `bundled` feature) as the container.

- `bundled` so there is no system SQLite dependency and behaviour is identical on
  all three platforms.
- WAL journalling; `synchronous = FULL` on the save transaction.
- Slot metadata lives in real columns so the save-slot screen can render without
  deserializing a game.
- The save transaction acquires an advisory lock (`fs4`) first, so a second
  process reports `SaveError::Locked` rather than corrupting the file.

What is *stored* in it is decided separately — see ADR 0008.

## Consequences

- Transactional durability and crash safety come from a well-tested C library
  rather than from hand-rolled rename dances.
- Adds a C toolchain requirement to the build. Verified working on Windows,
  Linux and macOS during Phase 0.
- `libsqlite3-sys` appears in the dependency closure of `persistence` only; the
  guard in ADR 0002 rejects it anywhere in the core.

## Alternatives considered

- **File per slot (msgpack + rename).** Rejected: no slot querying, and backup
  and multi-slot bookkeeping become bespoke code.
- **`sled` / `redb`.** Rejected: less operational history for a file players will
  care about losing, and no ad-hoc query story for the derived read-model.
