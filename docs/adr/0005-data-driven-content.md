# ADR 0005 — Content is data, validated at load, interned into a registry

- **Status:** Accepted
- **Date:** 2026-08-02

## Context

Species, evolution trees, items, moves, illnesses, quests and balance numbers all
change far more often than the rules that consume them. Compiling them in means a
designer needs a Rust build to retune a food item, and a typo becomes a runtime
panic in front of a player.

## Decision

- Content is authored as RON under `assets/content/`, loaded by
  `crates/content`.
- `content::definitions` mirrors the on-disk shape in *designer* units — decimals
  and human-readable ids. These are not domain types.
- Loading is three stages: **parse** → **validate** → **intern**. Validation
  checks reference integrity, evolution-graph acyclicity and reachability,
  numeric ranges, symmetry of the breeding compatibility matrix, id uniqueness,
  and localization coverage. Each rule has a test in `tests/content/`.
- After validation, ids resolve to indices in an immutable `ContentRegistry`.
  **Content lookup cannot fail at runtime**; there is no `Option` to unwrap.
- The parse → domain conversion is the single place authored decimals become
  `domain::units::Fixed` (ADR 0006). It is snapshot-tested.
- Balance lives in `config/*.ron`, never in code.

`cargo run -p simulation-cli -- validate-content` is the designer feedback loop
and a required CI check; a path-filtered workflow runs content tests alone so it
returns in under a minute.

## Consequences

- Content errors are caught in CI with a file and line, not at runtime.
- Modding and localization become mostly free.
- Cost: two parallel type hierarchies (definition and domain) plus a conversion
  layer. Accepted — that conversion is precisely where unit discipline is
  enforced.

## Alternatives considered

- **JSON.** Rejected for authoring: no comments, noisier for hand-written data.
  Retained for tool interchange.
- **Compile-time content via `include_str!` + macros.** Rejected: no designer
  iteration without a Rust toolchain.
