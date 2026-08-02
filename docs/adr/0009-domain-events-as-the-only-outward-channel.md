# ADR 0009 — Operations return domain events; events are the only outward channel

- **Status:** Accepted
- **Date:** 2026-08-02

## Context

A pet eating must update needs, possibly trigger an evolution check, advance a
quest objective, award an achievement, play an animation, play a sound, and be
recorded by telemetry. Wiring each consumer into the eat logic makes gameplay
code depend on quests, audio and analytics — and makes it impossible to test the
eat rule in isolation.

## Decision

Every domain and simulation operation has the shape:

```rust
fn apply(&mut self, cmd: Command, ctx: &Ctx) -> Result<Vec<DomainEvent>, DomainError>;
```

- `DomainEvent` is the **only** channel out. Presentation, telemetry, quest
  progress and achievement progress are all event consumers; none is called from
  gameplay code.
- Events are values: serializable, comparable, and cheap to assert on. "Feeding a
  hungry adult emits exactly these three events" is a one-line test.
- `DomainEvent` and its variants are `#[non_exhaustive]` so adding one is not a
  breaking change for consumers.
- Combat is fully replayable from `(WorldSeed, BattleId, Vec<PlayerAction>)`
  together with ADR 0007. `battle_history` therefore stores seeds and actions,
  not turn logs.

## Consequences

- Quests and achievements can be added in Phase 7 without touching Phase 3
  simulation code.
- `fuzz_targets/resolve_battle.rs` is straightforward: generate actions, replay,
  assert no panic and a well-formed result.
- Battle history is tiny and can be re-rendered at any fidelity later.
- Cost: an allocation per operation for the event vector, and indirection when
  tracing a behaviour end to end. Mitigated with `SmallVec` on the hot path;
  event-name search is a good enough substitute for a call graph.

## Alternatives considered

- **Direct calls into consumers.** Rejected: couples gameplay to UI, audio and
  analytics.
- **A global event bus with dynamic subscribers.** Rejected: ordering becomes
  implicit, which conflicts with the determinism requirement of ADR 0003.
