# ADR 0003 — Fixed-step simulation, one tick per game-minute, no offline approximation

- **Status:** Accepted
- **Date:** 2026-08-02

## Context

A Tamagotchi is defined by what happens while the player is away. `init-plan.md`
requires a nightly *offline/live simulation equivalence* check. The usual design
— a cheap closed-form "catch-up" that approximates N hours of decay, alongside a
per-frame live loop — makes that check impossible to write: the two paths are
different code producing deliberately different results.

## Decision

- One tick is one game-minute. `TICKS_PER_HOUR = 60`.
- The simulation advances only in whole ticks. Variable timesteps do not exist.
- **Offline catch-up runs the same function as the live loop**, for the exact
  number of elapsed ticks. There is no second code path to approximate.
- System execution order is a fixed `const` array in `simulation::scheduler`,
  not registration order.
- Budget: ≤ 1 µs per pet per tick, enforced by `benches/simulation.rs`.

At that budget a one-year absence (525,600 ticks, 8 pets) catches up in roughly
4 seconds — inside a loading screen with a progress bar. Total catch-up is capped
at two years; beyond that, elapsed time is clamped and
`SaveMetadata::clamped_offline_ticks` records it so the UI can explain what
happened.

## Consequences

- `tests/simulation/offline_equivalence.rs` becomes a trivially strong test:
  `live(N) == offline(N)`, exact equality.
- The per-tick budget is a hard constraint on system design; anything expensive
  must be event-driven or amortised rather than run every tick.
- Long absences cost CPU rather than accuracy. Accepted: correctness of the
  central pet-care fantasy outranks a few seconds of load time.

## Alternatives considered

- **Closed-form offline approximation.** Rejected: unverifiable, and every
  balance change silently desynchronises it from the live loop.
- **Coarser ticks (one hour).** Rejected: too coarse for illness onset, sleep
  cycles and battle cooldowns to feel responsive.
