# Architecture Decision Records

One file per decision that is expensive to reverse. Each records the context, the
decision, the consequences we accepted, and the alternatives we rejected.

An ADR is never edited to change its decision. To reverse one, add a new ADR that
supersedes it and mark the old one `Superseded by NNNN`.

| # | Decision | Status |
|---|---|---|
| [0001](0001-use-workspace.md) | Build as a Cargo workspace | Accepted |
| [0002](0002-domain-independent-from-bevy.md) | Core must not depend on Bevy, SQLite or the OS — enforced in CI | Accepted |
| [0003](0003-use-fixed-step-simulation.md) | Fixed-step simulation, one tick per game-minute, no offline approximation | Accepted |
| [0004](0004-use-sqlite.md) | SQLite as the save container | Accepted |
| [0005](0005-data-driven-content.md) | Content is data, validated at load, interned into a registry | Accepted |
| [0006](0006-integer-only-simulation-math.md) | Integer-only simulation math; floats banned in the core | Accepted |
| [0007](0007-counter-based-rng.md) | Counter-based RNG derived from position in state space | Accepted |
| [0008](0008-snapshot-authoritative-persistence.md) | Snapshot authoritative, SQL tables derived | Accepted |
| [0009](0009-domain-events-as-the-only-outward-channel.md) | Operations return domain events; events are the only outward channel | Accepted |

0006, 0007 and 0008 are the load-bearing ones: they are what make the
offline/live and save/load equivalence tests possible, and they are the hardest
to retrofit. Read those first.
