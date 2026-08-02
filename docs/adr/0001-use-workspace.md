# ADR 0001 — Build Rustagotchi as a Cargo workspace

- **Status:** Accepted
- **Date:** 2026-08-02

## Context

Rustagotchi has to run as a desktop game, as a headless simulator for balance and
CI, and eventually — per the closing note of `docs/init-plan.md` — on other
clients. A single crate would make the "no UI in the core" rule unenforceable:
`cargo` has no intra-crate visibility mechanism that can stop a UI module from
importing simulation internals.

## Decision

One workspace, ten library crates and three application binaries, with crate
boundaries drawn along the layering in `docs/init-plan.md`.

Crate names are `rustagotchi-*`; each sets `[lib] name` to the unprefixed form so
call sites read `use domain::pet::Pet`.

The default member set is the headless core. `crates/presentation`,
`apps/desktop` and `apps/content-editor` are excluded so that everyday
`cargo check` and `cargo test` never build the graphics stack.

## Consequences

- Layering violations become compile errors, not review comments.
- `cargo test -p rustagotchi-domain` is fast and has no I/O, which is what makes
  the property-test-heavy strategy affordable.
- Cost: more `Cargo.toml` files, and cross-crate refactors touch more places.
  Accepted — the alternative erodes silently.
- Compile-time risk is real (risk R6 in `docs/implementation-plan.md`); mitigated
  by the default member set and tracked with `cargo build --timings` at each
  phase exit.

## Alternatives considered

- **Single crate with modules.** Rejected: no enforcement.
- **Separate repositories per layer.** Rejected: version-lockstep overhead with
  no benefit at this team size.
