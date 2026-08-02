## What and why

<!-- One paragraph. What changes, and what problem it solves. -->

## Phase

<!-- Which phase of docs/implementation-plan.md does this belong to? -->

## Checklist

- [ ] `./scripts/check.sh` (or `check.ps1`) passes locally
- [ ] New behaviour has a test at the right layer (unit / property / integration / simulation)
- [ ] No new `f32`/`f64`/`HashMap` in `domain` or `simulation` (ADR 0006)
- [ ] No new randomness source outside `domain::rng` (ADR 0007)

### If the dependency closure changed

- [ ] `scripts/dep-allowlist/*.txt` regenerated and the diff is intentional

### If `GameState` or any serialized type changed

- [ ] `schema_version` bumped
- [ ] Migration function added
- [ ] Fixture added under `tests/fixtures/saves/`
- [ ] `tests/persistence/migration_compatibility.rs` passes against every fixture

### If simulation behaviour changed

- [ ] `deterministic_simulation` and `offline_equivalence` still pass
- [ ] Balance numbers changed in `config/*.ron`, not in code
