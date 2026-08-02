# Contributing

## Setup

```bash
./scripts/setup.sh      # ./scripts/setup.ps1 on Windows
```

Installs the pinned toolchain plus `cargo-nextest`, `cargo-deny`, `cargo-audit`
and `cargo-llvm-cov`. Add `--with-nightly` / `-IncludeNightly` if you are working
on fuzz targets.

## Before you open a pull request

```bash
./scripts/check.sh      # ./scripts/check.ps1 on Windows
./scripts/check.sh --fast   # during iteration: skips release build and supply chain
```

CI runs the same steps in the same order. `--fast` is for the inner loop only.

## The rules that are enforced, and why

These are not style preferences; each one exists because a specific test would
otherwise be impossible to write.

### Integer math in `domain` and `simulation` (ADR 0006)

`f32` and `f64` are rejected by Clippy in those crates. Use `Milli` for
quantities and `Fixed` for rates. Floats are fine in `presentation`.

*Why:* `assert_eq!(live_result, offline_result)` and
`assert_eq!(saved, loaded)` have to be exact equality. With float accumulation
over 525,600 ticks they cannot be.

### No ambient randomness (ADR 0007)

Never construct a generator. Call
`domain::rng::rng_for(seed, stream, tick, entity)`. Adding a new kind of roll
means adding an `RngStream` variant with a new discriminant — never renumbering
an existing one.

*Why:* RNG state stays out of the save file, and a combat balance tweak cannot
silently change which pets mutate.

### `BTreeMap`, not `HashMap`, in the core

Rejected by Clippy in `domain` and `simulation`.

*Why:* `HashMap` iteration order varies between runs, so anything that iterates
one becomes nondeterministic.

### Balance lives in `config/*.ron`, content in `assets/content/`

Not in code. If you find yourself typing a tuning number into a `.rs` file, it
belongs in a config file.

### Events are the only outward channel (ADR 0009)

Gameplay code returns `Vec<DomainEvent>`. It does not call into UI, audio,
quests, achievements or telemetry.

## Adding a dependency

The dependency closure of `domain`, `content`, `simulation` and `application` is
snapshot-tested. After adding a crate:

```bash
./scripts/check-deps.sh --update    # or: ./scripts/check-deps.ps1 -Update
```

Commit the resulting diff. Review it — a one-line addition that pulls in forty
packages is worth a conversation, and that is exactly what this makes visible.

## Changing anything serialized

Any change to `GameState` or a type reachable from it:

1. Bump `schema_version`.
2. Add `fn migrate_vN_to_vN1(..)`.
3. Commit a fixture save under `tests/fixtures/saves/`.
4. Confirm `tests/persistence/migration_compatibility.rs` passes — it iterates
   every fixture, so an old save that no longer loads fails CI.

Fixtures are kept forever. Deleting one deletes the evidence that a player's save
from that version still works.

## Testing

Put the test at the layer that makes it cheap to run and hard to break:

| Kind | Where | For |
|---|---|---|
| unit | `#[cfg(test)]` in the module | one function, one rule |
| property | in-crate, `proptest` | invariants over generated input |
| snapshot | `insta` | content conversion, event streams |
| integration | `tests/integration/` | flows across crates |
| persistence | `tests/persistence/` | durability, atomicity, migration |
| content | `tests/content/` | authored data coherence |
| simulation | `tests/simulation/` | determinism, equivalence, long horizons |

`crates/test-support` exists to make this cheap — builders, in-memory fixtures,
fake clock, seeded RNG, and `assert_state_eq` with a structural diff. Extend it
rather than re-deriving scaffolding in each suite.

Coverage gate: ≥ 85 % on `domain` and `simulation`. No target on `presentation`.

## Commit and PR conventions

- Conventional Commits (`feat:`, `fix:`, `refactor:`, `docs:`, `test:`,
  `chore:`), with the crate as scope where it helps: `feat(domain): add Milli`.
- One phase per PR where practical; note the phase in the PR description.
- Update `CHANGELOG.md` under `[Unreleased]` for anything user-visible or
  save-affecting.
