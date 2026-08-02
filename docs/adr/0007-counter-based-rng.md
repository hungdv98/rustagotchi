# ADR 0007 — RNG is a pure function of position in state space, not a stream

- **Status:** Accepted
- **Date:** 2026-08-02

## Context

The obvious design is one `StdRng` threaded through the simulation. It has three
properties that are actively harmful here:

1. **It is mutable state that must be saved.** Serializing generator internals
   couples the save format to an RNG implementation detail, and getting it wrong
   produces a save that loads but diverges.
2. **It couples unrelated systems.** Adding one accuracy roll to combat shifts
   every subsequent draw, so a combat balance tweak silently changes which pets
   mutate. Bisecting that is miserable.
3. **It makes a tick non-reproducible in isolation.** Reproducing tick 400,000 of
   a nightly failure requires replaying the preceding 399,999 ticks.

## Decision

Randomness is derived, never carried:

```rust
pub fn rng_for(seed: WorldSeed, stream: RngStream, tick: Tick, entity: u64) -> ChaCha8Rng;
```

- `RngStream` is an explicit enum with stable discriminants — `NeedsDecay`,
  `IllnessOnset`, `Mutation`, `GeneInheritance`, `CombatHit`, `CombatCrit`,
  `CombatAi`, `LootRoll`, `BehaviorChoice`, `WeatherRoll`, `QuestRoll`. New
  streams get new discriminants; existing ones are never renumbered.
- ChaCha8 for speed with good statistical quality and a specified, portable
  algorithm — the same seed produces the same bytes on every platform and every
  version of the crate.
- Entropy from the OS is used exactly once, to generate the `WorldSeed` for a new
  game, in `crates/platform`. `rand::rng` is banned in `simulation` by
  `clippy::disallowed_methods`.
- Identifiers are derived the same way, which is why the workspace `uuid`
  dependency does not enable the `v4` feature.

## Consequences

- RNG state never enters the save file; save/load equivalence is structural
  rather than something to test for.
- Balance changes stay local: adding a die roll to combat cannot perturb
  breeding.
- Any tick is reproducible standalone, so a nightly failure reduces to
  `rng_for(seed, stream, tick, entity)`.
- Cost: every call site must supply `(tick, entity)`. That is a small tax and it
  makes the dependency on simulation position explicit rather than hidden.
- Sequences within one `(stream, tick, entity)` are still ordinary generator
  draws; only the *starting point* is derived. Consuming a different number of
  draws inside one such cell still changes that cell's outcome — the isolation
  guarantee is between cells.

## Alternatives considered

- **One `StdRng` in `GameState`.** Rejected for the three reasons above.
- **Per-system RNG streams, each with saved state.** Fixes coupling but keeps
  state in the save and does not give standalone tick reproducibility.
