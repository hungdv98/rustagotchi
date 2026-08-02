# ADR 0002 — The core must not depend on Bevy, SQLite or the OS

- **Status:** Accepted
- **Date:** 2026-08-02

## Context

`docs/init-plan.md` states the rule: `domain` must not depend on Bevy, SQLite,
the filesystem, OS APIs, UI code, or application configuration. Stating a rule
is not enforcing one. Every codebase with this rule in its README and nowhere
else eventually violates it, usually via a transitive dependency nobody
inspected.

## Decision

Dependencies point inward: `presentation` → `application` → `domain`;
`persistence` and `platform` depend on `application` in order to *implement* its
port traits. `domain` depends on nothing outside `serde`, `thiserror`, `uuid`,
`smallvec` and `rand_chacha`.

Enforcement is mechanical, in CI, by two independent mechanisms:

1. **`scripts/check-deps.*`** compares the full non-dev dependency closure of
   `domain`, `content`, `simulation` and `application` against committed
   snapshots in `scripts/dep-allowlist/`. Any new package, direct or transitive,
   fails the build until the snapshot is regenerated — which puts the addition in
   a reviewable diff. A denylist backstop names the specific rule violated for
   packages such as `bevy`, `rusqlite`, `tokio` and `directories`.
2. **`crates/domain/clippy.toml`** and **`crates/simulation/clippy.toml`** ban
   `std::path`, `std::time::SystemTime`, `std::time::Instant` and the float
   types via `clippy::disallowed_types`, denied at workspace level.

At runtime the same boundary holds: no Bevy system mutates `GameState` directly;
UI sends `Command`s and reads `DomainEvent`s and DTOs.

## Consequences

- The core is testable with no window, no database and no clock — which is what
  makes Phases 1–5 verifiable before any pixel exists.
- Adding a legitimate dependency costs one extra step (`--update` and commit).
  That friction is the point.
- Verified during Phase 0: per-crate `clippy.toml` resolution and primitive-type
  bans both work on the pinned toolchain.

## Alternatives considered

- **Review discipline alone.** Rejected: the failure mode is a transitive edge
  nobody looks at.
- **A hand-maintained allowlist of package names.** Rejected: transitive
  closures are too large to enumerate by hand and rot immediately. The snapshot
  is generated, so it cannot be wrong.
