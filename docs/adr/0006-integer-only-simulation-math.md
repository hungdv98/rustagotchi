# ADR 0006 — Simulated quantities are integers; floats are banned in the core

- **Status:** Accepted
- **Date:** 2026-08-02

## Context

Two of the most valuable tests in the project — *save/load equivalence* and
*offline/live equivalence* — assert that two `GameState` values are the same.
With floating-point accumulation, "the same" stops being a usable notion:

- Reordering an addition changes the result.
- `-ffast-math`-style backend freedom, SIMD widths and FMA contraction differ
  across targets, so a save produced on one machine may not reproduce on another.
- Any tolerance-based comparison is a threshold that has to be tuned, and it
  hides real regressions below the threshold.

The simulation runs 525,600 ticks for a simulated year. Small float error
accumulates over that horizon in ways that are hard to distinguish from bugs.

## Decision

All simulated quantities are integers.

```rust
/// A quantity in thousandths of a unit. `Milli(1_000)` is one unit.
pub struct Milli(pub i32);

/// Fixed-point rate or multiplier, Q16.16.
pub struct Fixed(i64);
```

Needs and stats are percentages on the `Milli` scale, so a full bar is
`Milli(100_000)` and prints as `100.000`.

> **Correction (Phase 1).** The first draft of this ADR described `Milli` as
> "milliunits, `0..=10_000 == 0.0..=100.0%`", which is self-contradictory: 10,000
> steps across 100% is hundredths of a percent, not thousandths of a unit. The
> implemented scale is the one above — the stored integer reads directly as
> percent × 1000. Resolution is ample: draining a full bar over seven days is 10
> per tick, far from the truncation floor.

- Rounding is **truncation toward zero**, everywhere, stated once and tested once.
  Note this is *not* an arithmetic right shift, which floors toward negative
  infinity; `Fixed` multiplication therefore divides rather than shifts.
- Arithmetic saturates. A silent wrap in a save file is a corrupted pet.
- Arithmetic is checked or explicitly saturating; silent wrap is a bug.
- Designers still author decimals in RON. Conversion to `Fixed` happens exactly
  once, at content load time (ADR 0005), and is snapshot-tested.
- `f32`/`f64` are rejected by `clippy::disallowed_types` in `crates/domain` and
  `crates/simulation`, denied at workspace level.
- Floats remain fully available in `crates/presentation` — positions, easing and
  audio are not simulation state — and in balance *tooling* output.

## Consequences

- `assert_eq!(live, offline)` and `assert_eq!(saved, loaded)` are exact, total
  and portable across platforms.
- A simulation reproduces bit-identically from a seed on any machine, which makes
  a nightly failure a one-line repro rather than an investigation.
- Cost: gameplay code is wordier (`Fixed::apply` rather than `*`), and designers
  must think in milliunits when reading debug output. Accepted.
- Truncation is a design input, not an accident: decay rates must be chosen so
  that per-tick truncation does not round a slow effect to zero.

## Alternatives considered

- **`f64` with a comparison tolerance.** Rejected: hides regressions, needs
  per-field tuning, and is still not portable.
- **Rational / bignum arithmetic.** Rejected: blows the per-tick budget in
  ADR 0003.
- **`f32` with a documented "same platform only" guarantee.** Rejected: gives up
  cross-platform save portability, which the game needs.
