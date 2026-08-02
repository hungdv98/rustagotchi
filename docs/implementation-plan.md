# Rustagotchi — Detailed Implementation Plan

Companion to [init-plan.md](init-plan.md). That document defines *what the workspace looks like*.
This document defines *how it gets built*: the order, the decisions that must be locked first, the
concrete API surface per crate, the exit criteria per phase, and the enforcement mechanisms that keep
the architecture from eroding.

**Status:** implementation in progress — Phases 0–5 complete (2026-08-02)
**Baseline:** empty repository (only `docs/init-plan.md` exists), Rust 1.97.1 stable, Windows host

---

## 1. Corrections to the baseline plan

The structure in `init-plan.md` is sound and is adopted essentially as-is. Six items in it are stale
or internally inconsistent and are corrected here before any code is written.

| # | Issue in `init-plan.md` | Correction |
|---|---|---|
| C1 | `ron = "0.11"`, `rand = "0.9"` | Latest stable is `ron 0.12`, `rand 0.10`. `rand 0.10` pairs with `rand_chacha 0.10`; the 0.9→0.10 API change affects every RNG call site, so pin it correctly on day one. |
| C2 | `panic = "abort"` in `[profile.release]` | **Drop it.** It disables `catch_unwind`, which `crates/telemetry/crash_report.rs` needs; it breaks `#[should_panic]` under `cargo test --release`; and it breaks `cargo bench` and `cargo fuzz`. Use a `std::panic::set_hook` crash reporter instead. Binary-size savings are not worth losing all four. |
| C3 | `rust-version = "1.85"` | An application, not a published library, has no MSRV consumers. Pin the exact toolchain in `rust-toolchain.toml` (`1.97.1`) and set `rust-version` to match. One number, no MSRV CI job. |
| C4 | `apps/dedicated-simulation` | Overlaps `apps/simulation-cli` almost entirely and has no consumer until multiplayer exists. **Defer to post-1.0.** Its `runner.rs` / `metrics.rs` responsibilities live in `simulation-cli benchmark` for now. |
| C5 | `apps/content-editor` in the main workspace | A Bevy/egui editor pulls the full graphics stack into every `cargo check --workspace`. Move it behind an opt-in workspace member set (Phase 7) so core-loop iteration stays fast. |
| C6 | Normalized SQLite tables **and** a snapshot blob, both authoritative | Dual-write to two representations of the same truth is the single most reliable source of save-corruption bugs. See **D5**: the snapshot is authoritative; tables are a derived read-model. |

---

## 2. Decisions to lock before Phase 1

These are the choices that are expensive to reverse once 20k lines depend on them. Each has a
recommendation; each becomes an ADR under `docs/adr/`.

### D1 — Determinism is a hard contract, enforced by integer math

The nightly suite in `init-plan.md` demands *offline/live simulation equivalence* and
*save/load equivalence*. Both are only testable as **exact equality** if the simulation contains no
floating-point accumulation.

**Decision:** all simulated quantities are integers.

```rust
// crates/domain/src/units.rs  — as implemented in Phase 1
/// A quantity in thousandths of a unit. `Milli(1_000)` is one unit.
/// Needs and stats are percentages on this scale: a full bar is `Milli(100_000)`.
pub struct Milli(pub i32);

/// Fixed-point rate or multiplier, Q16.16.
pub struct Fixed(i64);

impl Fixed {
    pub const ONE: Fixed = Fixed(1 << 16);
    // Division, not `>>`: an arithmetic shift floors toward negative infinity,
    // which would round -3.5 to -4 and break the truncation rule below.
    // i128 intermediates so the product cannot overflow.
    pub const fn mul(self, rhs: Fixed) -> Fixed;
    pub const fn apply(self, v: Milli) -> Milli;
}
```

`f32`/`f64` are permitted **only** in `presentation` (positions, easing, audio) and in balance
*tooling* output. A clippy `disallowed-types` rule blocks them in `domain` and `simulation`.

**Consequence:** designers author rates as decimals in RON; the loader converts to `Fixed` once, at
load time, and the conversion is snapshot-tested. Rounding is defined as truncation-toward-zero
everywhere, stated once, tested once.

### D2 — Counter-based RNG, not a stateful stream

A mutable `StdRng` threaded through the simulation must be serialized into the save, and any change
in the *number* of draws anywhere silently reshuffles every later outcome. Instead, derive an RNG
from position in state space:

```rust
// crates/domain/src/rng.rs
#[derive(Copy, Clone, Serialize, Deserialize)] pub struct WorldSeed(pub u64);

#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(u16)]
pub enum RngStream {
    NeedsDecay = 1, IllnessOnset = 2, Mutation = 3, GeneInheritance = 4,
    CombatHit = 5, CombatCrit = 6, CombatAi = 7, LootRoll = 8,
    BehaviorChoice = 9, WeatherRoll = 10, QuestRoll = 11,
}

/// Pure function of (seed, stream, tick, entity). No state, nothing to save.
pub fn rng_for(seed: WorldSeed, stream: RngStream, tick: Tick, entity: u64) -> ChaCha8Rng;
```

**Consequences that fall out for free:**
- RNG state never enters the save file → save/load equivalence is structural.
- Adding a die roll to combat cannot perturb breeding outcomes → balance changes stay local.
- Any tick is reproducible in isolation → a failing nightly sim reduces to a one-line repro.

### D3 — One tick = one game-minute, full fidelity offline, no coarse-graining

**Decision:** `TICKS_PER_HOUR = 60`. Offline catch-up runs *the same loop* as live simulation, for
the exact number of elapsed ticks. No approximation path exists.

This is affordable if a tick is cheap. Budget: **≤ 1 µs per pet per tick** (enforced by
`benches/simulation.rs`). At that budget:

| Absence | Ticks | Catch-up cost (8 pets) |
|---|---|---|
| 1 day | 1,440 | ~12 ms |
| 30 days | 43,200 | ~350 ms |
| 1 year | 525,600 | ~4.2 s |

A one-year absence is the pathological case and still completes inside a loading screen with a
progress bar. **The alternative — a separate approximate offline path — makes
`offline_equivalence.rs` untestable by construction.** Do not build it.

Guardrail: cap total catch-up at 2 years of ticks; beyond that, clamp elapsed time and record
`SaveMetadata::clamped_offline_ticks` so the UI can explain it.

### D4 — Domain returns events; nothing mutates behind the caller's back

Every domain/simulation operation has the shape:

```rust
fn apply(&mut self, cmd: Command, ctx: &Ctx) -> Result<Vec<DomainEvent>, DomainError>;
```

Events are the *only* channel to presentation, telemetry, and quest/achievement progress. A battle is
fully replayable from `(WorldSeed, BattleId, Vec<PlayerAction>)`, which means `battle_history` stores
seeds and actions rather than turn logs, and `fuzz_targets/resolve_battle.rs` is trivial to write.

### D5 — Snapshot is authoritative; SQLite tables are a derived index

**Decision:** the save file is a single versioned envelope. Normalized tables exist only as a
query/read-model, rebuilt from the snapshot on load.

```text
save.db (SQLite)
├── save_slots            (slot id, name, envelope version, checksum, timestamps)
├── game_snapshots        (slot id, version, zstd(msgpack(GameState)), blake-style checksum)
└── <derived, rebuilt on load>
    pets, pet_genetics, pet_memories, pet_habits, pet_illnesses, pet_relationships,
    eggs, lineage, inventory, quest_progress, achievements, battle_history, world_state
```

- **Write path:** validate → serialize → write snapshot → refresh derived tables → checksum → commit.
  All inside one transaction; the derived tables can never disagree with the snapshot because they
  are regenerated, not incrementally patched.
- **Read path:** load snapshot, verify checksum, migrate if needed, rebuild derived tables.
- **Migration:** operates on the snapshot only (one function per version step). Table schema changes
  are a full rebuild, never a data migration — which eliminates the entire class of bug
  `0002_add_genetics.sql`-style hand-written migrations produce.

`init-plan.md`'s numbered `.sql` migrations become **schema definitions** for the derived model, and
snapshot migrations become Rust: `fn migrate_v3_to_v4(v3: SnapshotV3) -> SnapshotV4`.

### D6 — Save format: MessagePack + zstd + checksum

`rmp-serde` (compact, fast, schema-evolution-friendly with named fields) → `zstd` level 3 → checksum
over the compressed bytes. RON stays as the *content authoring* format only. Envelope:

```rust
pub struct SaveEnvelope {
    pub magic: [u8; 8],        // b"RGOTCHI\0"
    pub format_version: u16,   // envelope layout
    pub schema_version: u32,   // GameState shape; drives migrations
    pub created_by: SemVer,
    pub checksum: [u8; 32],
    pub payload: Vec<u8>,      // zstd(msgpack(GameState))
}
```

### D7 — Dependency direction is enforced mechanically, not by convention

Two mechanisms, both in CI:

1. **`scripts/check-deps.sh`** — asserts `cargo tree -p rustagotchi-domain --edges normal --prefix none`
   is a subset of an allowlist (`serde`, `thiserror`, `rand_chacha`, `smallvec`, and their transitive
   deps). Same check for `simulation` and `application`. This is the load-bearing one: it catches the
   accidental `bevy` edge on the PR that introduces it.
2. **`clippy.toml` `disallowed-types`** in `crates/domain/` and `crates/simulation/` — blocks
   `std::time::SystemTime`, `std::time::Instant`, `std::path::*`, `f32`, `f64`,
   `std::collections::HashMap`/`HashSet` (iteration-order nondeterminism; use `BTreeMap`/`BTreeSet`).

   **Verified in Phase 0** on toolchain 1.97.1: per-crate `clippy.toml` resolves correctly, primitive
   types (`f32`/`f64`) are valid `disallowed-types` paths, and the `reason` strings surface in the
   error. One caveat found: Clippy does **not** merge config files — the first one found wins — so any
   crate shipping its own `clippy.toml` must repeat the workspace-root settings. Both crate-local
   files do.

### D8 — Test runner and naming

`cargo-nextest` for all test execution (parallel, per-test isolation, better CI output). Crate names
are `rustagotchi-{domain,application,...}` with `[lib] name = "..."` unprefixed for ergonomic
`use domain::...`.

---

## 3. Toolchain and pinned dependencies

Verified against crates.io on 2026-08-02. Re-verify at the moment of pinning.

```toml
# rust-toolchain.toml
[toolchain]
channel = "1.97.1"
components = ["rustfmt", "clippy", "rust-src"]
targets = ["x86_64-pc-windows-msvc", "x86_64-unknown-linux-gnu", "aarch64-apple-darwin"]
```

| Crate | Version | Used by | Notes |
|---|---|---|---|
| `serde` | 1.0.229 | all | `derive` |
| `rmp-serde` | 1.3.1 | persistence | save payload |
| `zstd` | 0.13.3 | persistence | payload compression |
| `sha2` | 0.11.0 | persistence | envelope checksum |
| `ron` | 0.12.2 | content | **not** 0.11 (C1) |
| `serde_json` | 1.0 | content, tools | editor interchange |
| `thiserror` | 2.0.19 | libraries | typed errors |
| `anyhow` | 1.0.104 | apps only | never in library crates |
| `rand` | 0.10.2 | simulation | traits only |
| `rand_chacha` | 0.10.0 | domain | ChaCha8 for D2 |
| `uuid` | 1.24.0 | domain | v4 + serde |
| `chrono` | 0.4.45 | platform, persistence | wall-clock only; never in sim |
| `tracing` | 0.1.44 | all | |
| `tracing-subscriber` | 0.3.23 | apps | `env-filter`, `json` |
| `rusqlite` | 0.40.1 | persistence | `bundled` feature |
| `fs4` | 1.1.0 | platform | advisory save lock |
| `directories` | 6.0.0 | platform | per-OS save paths |
| `notify-rust` | 4.18.0 | platform | desktop notifications |
| `clap` | 4.6.5 | apps | `derive` |
| `bevy` | 0.19.0 | presentation, desktop | Phase 6 — re-verify then |
| `bevy_egui` | 0.41.1 | content-editor | **verify bevy 0.19 compat at Phase 7** |
| `fluent` | 0.17.0 | content | `.ftl` localization |
| `proptest` | 1.11.0 | tests | invariants |
| `insta` | 1.48.0 | tests | snapshot tests |
| `criterion` | 0.8.2 | benches | |
| `tempfile` | 3.27.0 | test-support | |
| `arbitrary` | 1.4.2 | fuzz | |

Dev tooling: `cargo-nextest`, `cargo-deny`, `cargo-audit`, `cargo-fuzz` (nightly), `cargo-llvm-cov`.

---

## 4. Phase plan

Each phase ends with an **executable** exit criterion — a command that passes or fails. No phase is
"done" because the files exist.

| Phase | Name | Delivers | Est. |
|---|---|---|---|
| 0 | Foundations | workspace, toolchain, CI, ADRs, guard scripts | **done** |
| 1 | Domain core | pure types, invariants, events | **done** |
| 2 | Content pipeline | RON definitions, loader, registry, validator | **done** |
| 3 | Simulation core | fixed-step loop, needs/health/aging, offline | **done** |
| 4 | Application + persistence | commands/ports/handlers, save/load/migrate | **done** |
| 5 | Breeding, genetics, combat | full headless vertical slice | **done** |
| 6 | Presentation + platform + desktop | playable Bevy client | 4–6 wk |
| 7 | Depth | minigames, quests, economy, l10n, telemetry, editor | 4–6 wk |
| 8 | Release | packaging, signing, installers, release CI | 2–3 wk |

Estimates assume one focused engineer and are ranges, not commitments; Phases 1–5 are the ones with
low variance because they are pure logic with mechanical exit criteria.

---

### Phase 0 — Foundations ✅ complete (2026-08-02)

**Built**

- Root `Cargo.toml` per `init-plan.md` §"Root workspace configuration", with corrections C1–C5
  applied. `resolver = "3"`, `edition = "2024"`.
- All 10 library crates + `apps/simulation-cli` created as compiling stubs (`lib.rs` with a doc
  comment and nothing else). `apps/desktop`, `apps/content-editor` created but **excluded** from
  default members.
- `rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml` (workspace + per-crate), `deny.toml`,
  `.editorconfig`, `.gitignore`.
- `scripts/check.ps1` + `scripts/check.sh`: fmt → clippy → nextest → check-deps → deny.
- `scripts/check-deps.sh`: the D7 allowlist guard.
- `.github/workflows/ci.yml`: matrix `{windows-latest, ubuntu-latest, macos-latest}` × `stable`,
  running `scripts/check.sh`, with `Swatinem/rust-cache`.
- ADRs 0001–0005 from `init-plan.md`, **plus** 0006 (integer determinism, D1), 0007 (counter RNG, D2),
  0008 (snapshot-authoritative persistence, D5).

**Exit criteria — all met**

```bash
cargo check --workspace --all-targets   # green (local: Windows/MSVC; CI matrix covers Linux + macOS)
./scripts/check.ps1                     # fmt, clippy -D warnings, deps, test, deny, audit, release: all pass
./scripts/check.sh --fast               # bash path verified under Git Bash
./scripts/check-deps.sh                 # PowerShell and bash implementations agree byte-for-byte
cargo run -p simulation-cli -- --help   # command surface present
```

**Guards proved by negative test, not just by passing.** Both were made to fail on purpose and then
reverted:

- Adding `f64` and `HashMap` to `crates/domain` → 5 Clippy errors carrying the ADR 0006 rationale.
- Adding `rusqlite` to `crates/domain` → the dependency guard named the violated rule
  (`Storage is an adapter behind application::ports::SaveRepository`) for both `rusqlite` and the
  transitive `libsqlite3-sys`, and flagged the propagation into `content`, `simulation` and
  `application`.

**Findings worth carrying forward**

| Finding | Resolution |
|---|---|
| `rusqlite` + `zstd` `bundled` features need a C toolchain | Verified working on this Windows/MSVC host; the CI matrix is what proves Linux and macOS. Documented as a prerequisite in the README. |
| `cargo deny` reads internal `path` deps as wildcard versions | `publish = false` on every internal crate + `allow-wildcard-paths = true`. A wildcard on a real registry dependency still fails. |
| `option-ext` (via `directories`) is MPL-2.0 | Allowed with a written rationale in `deny.toml`. The allow-list is kept to licences actually in the graph, so unused entries do not train people to ignore the warnings. |
| Windows PowerShell 5.1 `-Encoding utf8` writes a BOM + CRLF | The snapshot writer uses BOM-less UTF-8 with LF, and `.gitattributes` pins `*.sh` and the snapshots to LF. Without this the bash guard fails on every line while the PowerShell one passes. |
| `$ErrorActionPreference = 'Stop'` aborts on native stderr | `check.ps1` runs under `Continue` and judges success by `$LASTEXITCODE`. |

**Deferred deliberately:** `content-validation.yml` and `nightly-simulation.yml` arrive with Phases 2
and 3 — a workflow that references tests which do not exist yet is a red build teaching people to
ignore red builds. The `coverage` job exists in `ci.yml` behind `if: false` with a `TODO(Phase 1)`.

---

### Phase 1 — Domain core ✅ complete

Pure logic. Zero I/O, zero clock, zero randomness *source* (RNG is derived per D2, from values passed in).

| Module | Contents |
|---|---|
| `units` | `Milli`, `Fixed` (Q16.16), truncate-toward-zero, saturating arithmetic, exact decimal parse/print |
| `rng` | `WorldSeed`, `RngStream`, `rng_for`, unbiased `below`/`range_inclusive`/`chance`/`shuffle` |
| `identifiers` | `ContentKey` + 8 content ids; 4 derived UUIDv8 runtime ids |
| `time` | `Tick`, `GameDuration`, `UnixSeconds`, `GameClock` with remainder carry-forward |
| `error`, `validation` | `DomainError`, `ValidationError`, `snake_case` id grammar |
| `events` | `DomainEvent`, `PetEvent`, `DeathCause`, `DomainEvents` (`SmallVec<[_; 4]>`) |
| `pet` | `Pet` aggregate + `LifeStage`, `PhysicalNeeds`, `Stats`, `Personality`, `EmotionalState`/`Mood`, `ActivityState`, `HealthState`, `PetProgression`, `Memories`, `Habits`, `Relationships` |
| `genetics` | `Allele`/`Dominance`, `GenePair`/`Expression`, `GeneticProfile`, `Phenotype`, `Mutation`/`MutationRate`, `Ancestry`/`Parentage` |
| `inventory` | `Inventory`, `ItemStack`, `Equipment`, `ItemCategory`/`EquipmentSlot` |
| `economy` | `Currency`, `Wallet`, `Transaction`/`TransactionReason` |

Also landed: `crates/test-support` (`assert_state_eq` with a structural diff, `PetBuilder`,
`NeedsBuilder`, `FakeClock`, `seeded_rng`) and the `tests/` package described below.

**321 tests pass; 5 are skipped** — the Phase 3 determinism contracts, which compile against
`tests/src/harness.rs` and are `#[ignore]`d rather than absent.

**Two design rules that emerged and are worth keeping:**

- *Invariants live in the types.* `NeedValue`, `StatValue`, `TraitValue`, `Affinity`, `Level` and
  `StatValue` all clamp on construction, so the `Pet` aggregate's fields can be public — direct field
  access is convenient for the Phase 3 systems and still cannot produce an impossible value. Only the
  operations spanning several fields (`set_need`, `advance_stage`, `die`) are methods, because only
  those have an invariant no single type can hold.
- *Every collection is bounded, with a documented eviction rule.* Memories (32), relationships (64),
  habits (decay-to-forget) and ailments (8). A pet simulated for a year takes 525,600 ticks; anything
  unbounded is a save that grows forever.

**Deliberately deferred out of Phase 1:**

| Item | Moved to | Why |
|---|---|---|
| `pet/skills` | Phase 5 | A skill is a learned combat move; it has nothing to model until `domain::combat` exists. |
| `pet/illnesses` as its own module | folded into `pet/health` | An ailment is only meaningful attached to health; two modules would have shared every invariant. |
| `economy/shop`, `economy/pricing` | Phase 7 | Both need the content registry to mean anything. |

**Structural fix made along the way.** `init-plan.md` places the cross-crate suites at a bare
root-level `tests/` directory. That cannot work: integration tests belong to a package, and the
workspace root is a virtual manifest. `tests/` is now a package (`rustagotchi-tests`) with
`autotests = false` and explicit `[[test]]` entries, which preserves the intended
`simulation/`, `integration/`, `content/`, `persistence/` grouping — Cargo does not discover test
files in subdirectories on its own.

**Deviations worth knowing about:**

- `rand_core` 0.10 renamed `RngCore` to `Rng`, which collides with our own `Rng`. Imported
  anonymously (`use rand_chacha::rand_core::Rng as _`) so the generator handle the rest of the
  codebase sees stays ours.
- The `Milli` scale in the D1 snippet above was self-contradictory; corrected there and in ADR 0006.
- `uuid` keeps `default-features = false` and no `v4`. Runtime ids are derived UUIDv8 values, which is
  what makes a replay reproduce the identical id graph.

**Files** (`crates/domain/src/`): `error.rs`, `identifiers.rs`, `units.rs` *(new — D1)*, `rng.rs`
*(new — D2)*, `validation.rs`, `time/*`, `pet/*`, `genetics/*`, `inventory/*`, `economy/*`, `events/*`.
Deferred to Phase 5: `breeding/*`, `combat/*`. Deferred to Phase 7: `quests/*`, `achievements/*`,
`world/*`.

**Key surface**

```rust
// identifiers.rs — newtype every id; never pass a bare Uuid or String
macro_rules! id { ($n:ident) => { #[derive(Copy,Clone,PartialEq,Eq,Hash,Ord,PartialOrd,Serialize,Deserialize)] pub struct $n(Uuid); } }
id!(PetId); id!(EggId); id!(BattleId); id!(SaveSlotId);
// Content ids are interned strings, validated at load: SpeciesId, ItemId, MoveId, QuestId, IllnessId

// time/game_time.rs
pub const TICKS_PER_HOUR: u32 = 60;
pub const TICKS_PER_DAY: u32 = TICKS_PER_HOUR * 24;
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)] pub struct Tick(pub u64);
#[derive(Copy, Clone)] pub struct GameDuration(pub u64);
pub struct GameClock { pub current: Tick, pub started_at_wall: UnixSeconds }

// pet/needs.rs — every need is a Milli with an explicit floor/ceiling
pub struct PhysicalNeeds { pub hunger: Milli, pub energy: Milli, pub hygiene: Milli, pub bladder: Milli, pub social: Milli, pub fun: Milli }
impl PhysicalNeeds { pub fn clamp(&mut self); pub fn is_critical(&self) -> bool; }

// pet/entity.rs — the aggregate from init-plan.md §"Recommended internal domain boundaries"
pub struct Pet { /* identity, species, stage, needs, emotions, personality,
                    genetics, progression, breeding, combat, health, memories, habits */ }

// events/domain_event.rs
#[non_exhaustive]
pub enum DomainEvent { Pet(PetEvent), Breeding(BreedingEvent), Combat(CombatEvent), World(WorldEvent) }
```

**Invariants to encode (and property-test)**

1. `Milli` needs always land in `0..=10_000` after any operation.
2. `LifeStage` transitions are a DAG; no stage regresses.
3. A `Pet`'s `GeneticProfile` has exactly the gene loci its species declares.
4. `Personality` traits sum/normalize to a documented range and never drift under repeated updates.
5. Serialization round-trips: `msgpack(x) -> x` for every domain type (`proptest` + `Arbitrary`).

**Exit criteria**

```bash
cargo nextest run -p rustagotchi-domain          # incl. proptest invariants 1–5
./scripts/check-deps.sh                          # domain deps ⊆ {serde, thiserror, rand_chacha, uuid, smallvec}
cargo clippy -p rustagotchi-domain -- -D warnings # disallowed-types blocks f32/f64/SystemTime/HashMap
```

---

### Phase 2 — Content pipeline ✅ complete (2026-08-02)

**Files:** `crates/content/src/{lib,error,manifest,loader,registry,validator,cache}.rs`,
`definitions/*`, `formats/ron.rs`. `localization/*` deferred to Phase 7.

**Design**

- `definitions/*` are `#[derive(Deserialize)]` structs that mirror the RON on disk, using *designer
  units* (decimals, human ids). They are **not** domain types.
- `loader.rs` parses → `validator.rs` checks → conversion produces domain types with `Fixed`/`Milli`.
  This is the single place decimals become integers (D1).
- `registry.rs` is the loaded, validated, immutable world: `ContentRegistry` with
  `get_species(SpeciesId) -> &SpeciesDef` etc. Interned ids resolve to indices; lookups are `O(1)`
  and cannot fail after validation.

**Validator rules** (each is a test in `tests/content/`)

| Rule | Failure mode it prevents |
|---|---|
| Every referenced `ItemId`/`MoveId`/`SpeciesId` exists | dangling ref → runtime panic |
| Evolution graph is acyclic and every node reachable from a base species | unreachable content |
| Every species' move pool is non-empty at every stage | pet cannot act in combat |
| Breeding compatibility matrix is symmetric and total | asymmetric breeding results |
| No duplicate ids across any definition kind | silent last-write-wins |
| Numeric fields within declared min/max | designer typo → balance explosion |
| Every id has a localization key in `en-US` | untranslated string shipped |

**Authored content for this phase:** the 3 species from `init-plan.md` (`emberling`, `stormling`,
`aquafin`), 2 evolution trees, ~12 foods, ~8 medicines, ~16 combat moves, ~6 illnesses. Enough to
exercise every validator rule and every simulation branch.

**Exit criteria**

```bash
cargo run -p simulation-cli -- validate-content   # exits 0, prints a per-kind count table
cargo nextest run -p rustagotchi-content
# plus: a deliberately broken fixture pack fails with a precise, line-numbered error
```

All exit criteria are met. The base pack contains 3 species, 2 evolution edges, 20 consumables,
16 moves, 6 illnesses, a total symmetric 3×3 breeding matrix, and 45 en-US strings. The loader is
manifest-driven and size-bounded; it rejects unsafe relative paths and unknown RON fields, aggregates
semantic errors with stable codes, and preserves line/column positions for parse failures. A
path-filtered `content-validation.yml` keeps authored-content feedback independent of the full matrix.

---

### Phase 3 — Simulation core ✅ complete (2026-08-02)

**Files:** `crates/simulation/src/{lib,config,simulation_engine,scheduler,fixed_step,offline}.rs`,
`systems/{needs_decay,emotional_update,health_update,illness_update,aging,growth,evolution,cooldowns,habits,memory_decay}.rs`.
`behavior/*` in this phase; `combat/*` and `breeding/*` in Phase 5.

**Design**

```rust
pub trait SimSystem {
    const ID: SystemId;               // stable, used for ordering + profiling
    fn run(&self, ctx: &mut TickContext<'_>);
}

pub struct TickContext<'a> {
    pub tick: Tick,
    pub state: &'a mut GameState,
    pub content: &'a ContentRegistry,
    pub seed: WorldSeed,
    pub events: &'a mut Vec<DomainEvent>,
    pub config: &'a SimConfig,
}
```

- `scheduler.rs` holds a **fixed, explicit system order** (a `const` array, not registration order).
  Reordering is a deliberate, reviewed, save-schema-relevant change.
- `fixed_step.rs` accumulates real elapsed time and drains it in whole ticks. Never partial.
- `offline.rs` is `for tick in last..now { fixed_step::run_one(tick, ...) }` — literally the same
  function the live loop calls (D3). It differs only in that it batches events and reports progress.
- `SimConfig` is loaded from `config/simulation.ron`; balance numbers never live in code.

**Exit criteria**

```bash
cargo nextest run --test deterministic_simulation   # same seed → byte-identical GameState, 10k ticks
cargo nextest run --test offline_equivalence        # live(N ticks) == offline_catchup(N ticks), exact
cargo nextest run --test seven_day_simulation       # a neglected pet dies; a cared-for pet evolves
cargo nextest run --test one_year_simulation        # 525,600 ticks, no panic, no overflow, no NaN
cargo bench --bench simulation                      # ≤ 1 µs / pet / tick (D3 budget)
```

All exit criteria are met. Live and offline progression call the same `fixed_step::run_one`; the
five formerly ignored equality contracts now run on every workspace test. The scheduled suite covers
10,000-tick determinism, exact day/year offline equivalence, neglected and cared-for seven-day
lifecycle branches, and a 525,600-tick stress run. Offline event reports are capped at 4,096 entries
and disclose the number coalesced, preventing a long absence from becoming an unbounded allocation.
The corrected release benchmark resets live state outside the timed body and measures approximately
0.24 µs per pet per tick on the development host, below the 1 µs budget.

`offline_equivalence` is the highest-value test in the entire repo. Write it first, before the
systems it validates.

---

### Phase 4 — Application + persistence ✅ complete (2026-08-02)

**Application** (`crates/application/src/`)

```rust
// ports/ — the only way the core touches the outside world
pub trait Clock { fn now_unix(&self) -> UnixSeconds; }
pub trait ContentRepository { fn load(&self) -> Result<ContentRegistry, ContentError>; }
pub trait SaveRepository {
    fn list_slots(&self) -> Result<Vec<SlotSummary>, SaveError>;
    fn save(&self, slot: SaveSlotId, state: &GameState) -> Result<SaveMetadata, SaveError>;
    fn load(&self, slot: SaveSlotId) -> Result<GameState, SaveError>;
    fn delete(&self, slot: SaveSlotId) -> Result<(), SaveError>;
}
pub trait NotificationService { fn notify(&self, n: Notification); }
pub trait Telemetry { fn record(&self, e: &DomainEvent); }

// game_engine.rs — the façade every app talks to
impl GameEngine {
    pub fn dispatch(&mut self, cmd: Command) -> Result<Vec<DomainEvent>, AppError>;
    pub fn query<Q: Query>(&self, q: Q) -> Q::Output;
    pub fn advance_to(&mut self, wall_now: UnixSeconds) -> Result<Vec<DomainEvent>, AppError>;
}
```

One handler per command family; handlers are pure over `(GameState, ContentRegistry, ports)`.
DTOs exist so `presentation` never sees a domain type — this is what keeps Phase 6 from leaking Bevy
concerns backwards.

**Persistence** (`crates/persistence/src/`) — implements D5/D6.

Save transaction, per `init-plan.md` §"Persistence model", made concrete:

```text
1. acquire exclusive advisory lock on <save-dir>/.lock          (fs4; fail fast w/ clear error)
2. validate GameState invariants                                 (reject before writing anything)
3. serialize → msgpack → zstd → checksum
4. BEGIN IMMEDIATE
5. upsert game_snapshots(slot, schema_version, payload, checksum)
6. DELETE + rebuild derived tables for this slot
7. upsert save_slots metadata
8. COMMIT
9. rotate backups: keep last 3 + one daily                       (copy file, atomic rename)
10. release lock
```

**Exit criteria**

```bash
cargo nextest run --test save_load_flow           # save → load → GameState equality, exact
cargo nextest run --test atomic_save              # kill between steps 4–8 → previous save intact
cargo nextest run --test corrupted_save_recovery  # bit-flipped payload → detected, backup restored
cargo nextest run --test concurrent_save_lock     # 2nd process gets SaveError::Locked, not corruption
cargo nextest run --test migration_compatibility  # v1 fixture loads under current schema
cargo run -p simulation-cli -- inspect-save <path>
```

Keep a `tests/fixtures/saves/` directory with one committed save per schema version, forever. Every
new `schema_version` adds a fixture and a migration test; `migration_compatibility` iterates the
directory so this cannot be forgotten.

All exit criteria are met. `GameEngine` owns validated command dispatch, offline advancement and
presentation-safe DTO queries behind explicit application ports. Saves use a versioned MessagePack
envelope compressed with zstd and protected by SHA-256, with the snapshot as the sole authority and
SQLite pet, inventory and world tables rebuilt transactionally as read models. Advisory locking,
three rotating backups, a daily backup, checksum-triggered recovery and committed schema fixtures
are covered by the Phase 4 integration suite. The CLI can create a save during simulation and inspect
or export a slot as JSON. Capped offline time is consumed exactly once, so discarded elapsed time is
not replayed on later launches.

---

### Phase 5 — Breeding, genetics, combat ✅ complete (2026-08-02)

This phase closes the vertical slice. Split into two tracks that can proceed in parallel.

**Implemented so far:** explicit breeding eligibility and lineage refusal reasons; deterministic
per-locus inheritance and catalog-driven mutation; saveable egg/incubation state; schema-v1 through
schema-v3 migrations with permanent fixtures; breed/hatch application commands and events;
transactional egg and active-battle read models; and a pure deterministic battle resolver covering
initiative, move/energy validation, accuracy, elemental integer damage, defeat and one-time rewards.
The committed RON vertical slice now executes feed → three-day advance → breed → incubate → hatch →
battle → reward → save → load with exact equality. Release scale runs complete 100,000 offspring at
a 49.98/50.02 allele split and 100,000 status-enabled mirror battles at a 49.55% first-player win
rate; all three AI moves remain near one-third usage. Burning, poison and stun have bounded saveable
state, deterministic application streams and end-of-turn processing. All four cargo-fuzz targets
compile independently; nightly Linux CI runs each for five minutes.

**5a — Breeding & genetics**
`domain/breeding/*`, `domain/genetics/*` (completion), `simulation/breeding/*`.

Flow per `init-plan.md` §"Example breeding flow", with the rules made explicit:
eligibility (stage, age, health, cooldown, not siblings within N generations) → lineage check
(inbreeding coefficient from `lineage.rs`, penalty curve in `config/breeding_balance.ron`) →
compatibility score → per-locus inheritance (`RngStream::GeneInheritance`) → mutation roll
(`RngStream::Mutation`) → species resolution (hybrid table) → `Egg` with `IncubationState`.

**5b — Combat**
`domain/combat/*`, `simulation/combat/*`.

Turn resolution: initiative (speed + tiebreak by `RngStream::CombatAi`) → validate move/energy →
accuracy roll → damage (elemental matrix × stats × move power, all integer) → status application →
end-of-turn status processing → win/loss/rewards. A battle is a pure function
`resolve(seed, battle_id, initial_state, actions) -> BattleResult + Vec<CombatEvent>` (D4).

**`apps/simulation-cli` completion:** `simulate`, `inspect-save`, `validate-content`, `benchmark`.

**Exit criteria — the headless vertical slice from `init-plan.md`, as one command:**

```bash
cargo run -p simulation-cli -- simulate --script tests/scripts/vertical_slice.ron
# create pet → feed → advance 3 days → breed → egg → hatch → battle → resolve → save → load → assert equal
```

Plus the nightly scale suite:

```bash
cargo run -p simulation-cli -- benchmark --offspring 100000   # no panic; gene distribution within tolerance
cargo run -p simulation-cli -- benchmark --battles 100000     # no panic; win rate 45–55% in mirror matches;
                                                              # no move with >70% or <5% usage under greedy AI
cargo fuzz run resolve_battle -- -max_total_time=300
cargo fuzz run generate_offspring -- -max_total_time=300
cargo fuzz run load_save -- -max_total_time=300
cargo fuzz run parse_content -- -max_total_time=300
```

**At the end of Phase 5 the game is complete and testable without a single pixel.** This is the
milestone that matters most; do not start Phase 6 before it holds.

---

### Phase 6 — Presentation, platform, desktop (4–6 weeks)

**Status (2026-08-02): in progress.** Concrete platform adapters (paths, clock,
secure seed generation, notifications, file/process locks and lifecycle) are
implemented. Bevy 0.19 is pinned and the desktop composition root now supports
first-run/relaunch bootstrap, offline/live progression, starter-pet creation,
pet-status care, inventory, a two-founder breeding/incubation/hatch flow, save
and save-on-exit through the single command bridge. The battle arena now uses
authored move pools and presents deterministic results and rewards. Save-slot
listing/switching and native critical-need/death notifications are wired,
including events produced during offline catch-up. Care, breeding, and battle
now share explicit, bounded pet selection; commands target the pets displayed
by the UI rather than implicitly choosing the first entries. Every menu uses
Bevy's accessible button widgets with screen-reader labels, deterministic tab
order, Enter/Space activation, pointer activation through the same event path,
and a visible keyboard focus ring. A shared responsive screen shell now centers
and wraps content down to a supported 360×540 minimum window, live home data
refreshes without re-entering the screen, and every screen exposes accessible
success/error feedback. Pet status now presents responsive, text-labeled need
meters, uses the unambiguous "Fullness" label, and disables care actions that
cannot succeed. Expected care rejections are friendly notices rather than
application errors and do not expose internal entity identifiers. Two-pet
selection cycles correctly across both pets, while a one-pet screen explicitly
disables cycling. Breeding now disables age-ineligible pairs before dispatch;
expected eligibility rejections are also sanitized notices. Background
clock polling deliberately preserves
actionable errors. Meaningful domain events now drive typed success, warning,
and impact cues, with short screen pulses and generated desktop tones. Main-menu
controls can mute sound or disable motion while preserving textual feedback;
their labels now expose the current state without long toggle wording. Pet
names are validated domain values, flow through creation and presentation, and
persist in schema 4 with migration coverage for existing saves and SQLite read
models. Home is now a compact named-pet dashboard with quick actions and
original transparent Emberling, Stormling, and Aquafin assets. Battle replaces
opaque auto-resolution with Power, Precision, and Efficiency tactics backed by
authored move power, accuracy, and energy tradeoffs. The application DTO
exposes the selected pet's activity, and the presentation state machine maps
all nine authored activities plus transient event reactions to reduced-motion-
aware UI transforms. Home uses a responsive, bounded gallery of up to eight
animated portraits. A runtime sampler records average and p95 frame time after
a 60-frame warm-up whenever eight pets are visible, and warns when p95 exceeds
16.67 ms. The three-OS headless smoke suite constructs all eight portrait nodes
and enforces the same average CPU-frame budget over 1,000 frames. Rendered GPU
verification is now automated by a deterministic native verifier that opens an
1100×720 window, waits for all eight portrait assets, captures GPU output, and
validates foreground plus Emberling pixel coverage before saving an artifact.
It passes on the current Windows host. The existing CI matrix now runs it on
Windows and Linux (Xvfb/Mesa) and uploads both frames; the Linux result remains
pending until the baseline is pushed. A headless Bevy smoke test now drives the real accessible button
activation path through onboarding, care, companion adoption, battle,
breeding, two-day incubation, hatching, save-on-exit, SQLite reload, and
post-reload catch-up. It runs in the existing three-OS CI matrix without
requiring a display server.

**`crates/platform`** first (small, unblocks everything): `paths.rs` (`directories`), `clock.rs`,
`random.rs` (seed generation only), `file_lock.rs`/`process_lock.rs` (`fs4`), `notifications.rs`
(`notify-rust`), `lifecycle.rs`. Each is a concrete impl of an `application` port.

**`crates/presentation`** — Bevy 0.19.

- `app_state.rs`: `States` enum matching `screens/` one-to-one.
- The engine boundary: `GameEngine` lives in a Bevy `Resource`; systems send `Command`s through an
  `EventWriter`, a single `dispatch_commands` system drains them and re-emits `DomainEvent`s as Bevy
  events. **No Bevy system mutates `GameState` directly** — this keeps the D7 dependency direction
  honest at runtime, not just at compile time.
- `view_models.rs`: `DomainEvent`/DTO → renderable state. Presentation reads DTOs, never `Pet`.
- Screen order: `loading` → `main_menu` → `save_slots` → `pet_creation` → `home` → `pet_status` →
  `inventory` → `breeding` → `battle` → the rest.
- `animation/state_machine.rs` drives sprite state from pet activity + events; `audio/` on top.

**`apps/desktop`** — `bootstrap.rs` wires ports to impls, `plugins/*` per `init-plan.md`.

**Exit criteria:** create → care → breed → hatch → battle → save → quit → relaunch → offline
progression applied correctly, entirely through the UI, on Windows and Linux. 60 fps with 8 pets.

---

### Phase 7 — Depth (4–6 weeks)

Parallelizable once Phase 6 lands: `domain/quests`, `domain/achievements`, `domain/world` +
`simulation/systems/{quest_progress,environment}` and their screens; `crates/minigames` (3 games,
each returning a `MinigameResult` the domain converts to needs/currency); `content/localization`
with `en-US`/`vi-VN`/`ja-JP` `.ftl` + `validate_localization` test; `crates/telemetry` (logging,
metrics, `performance.rs`, panic-hook crash reports — enabled by C2); `apps/content-editor`
(re-verify `bevy_egui` ↔ `bevy` compatibility at start of this phase).

### Phase 8 — Release (2–3 weeks)

`packaging/` for Windows (WiX), Linux (AppImage + deb + desktop entry), macOS (bundle +
entitlements); `tools/release-builder`; `.github/workflows/release.yml` (tag-triggered, 3-OS matrix,
artifacts + checksums); `scripts/generate-release-notes.sh` from `CHANGELOG.md`; code signing and
notarization (needs certificates — start procurement during Phase 6, it has external lead time).

---

## 5. Testing strategy

| Layer | Location | What it proves | Gate |
|---|---|---|---|
| Unit | `#[cfg(test)]` in-crate | one function, one rule | every PR |
| Property | in-crate, `proptest` | invariants hold over generated input | every PR |
| Snapshot | `insta` | content→domain conversion, event streams stable | every PR |
| Integration | `tests/integration/` | flows across crates | every PR |
| Persistence | `tests/persistence/` | durability, atomicity, migration | every PR |
| Content | `tests/content/` | authored data is coherent | every PR + content-only workflow |
| Simulation | `tests/simulation/` | determinism, equivalence, long-horizon | PR (short) + nightly (long) |
| Bench | `benches/` | perf budgets (D3) | nightly, regression-gated |
| Fuzz | `fuzz/` | no panic on hostile input | nightly, 5 min/target |

**`crates/test-support`** exists so this is cheap: `builders.rs` (`PetBuilder::new().hungry().adult().build()`),
`fixtures.rs` (a minimal `ContentRegistry` that doesn't touch disk), `fake_clock.rs`, `seeded_rng.rs`,
`in_memory_repository.rs`, `assertions.rs` (`assert_state_eq` with a readable structural diff — this
one pays for itself the first time `save_load_flow` fails).

Coverage target: **≥ 85 % on `domain` + `simulation`**, measured by `cargo-llvm-cov`, enforced in CI.
No target on `presentation`.

---

## 6. CI

**`ci.yml`** (every PR): fmt → clippy `-D warnings` → `check-deps` (D7) → nextest → `cargo deny check`
→ `cargo audit` → `validate-content` → release build. Matrix: 3 OSes. ~12 min target with caching.

**`content-validation.yml`**: path-filtered to `assets/content/**` — runs only the content tests, so
designers get feedback in under a minute.

**`nightly-simulation.yml`**: the `init-plan.md` nightly list — 7-day lifecycle, 1-year accelerated,
100k offspring, 100k battles, save/load equivalence, offline/live equivalence, evolution graph
reachability, content reference validation, all 4 fuzz targets, criterion with regression detection.
Failure opens an issue automatically.

**`security.yml`**: `cargo audit` + `cargo deny` on a schedule (dependency advisories land between PRs).

**`release.yml`**: tag-triggered, Phase 8.

---

## 7. Risk register

| # | Risk | Impact | Mitigation |
|---|---|---|---|
| R1 | Determinism breaks silently; nightly equivalence tests start failing intermittently | Very high — undermines the entire save/offline model | D1 (integer math) + D2 (counter RNG) + `HashMap` ban. Write `offline_equivalence` **before** the systems it tests. |
| R2 | Scope: 14 crates, ~200 files, before any of it runs | High | Phases 1–5 are headless and mechanically verifiable. Nothing in Phase 6+ starts until the Phase 5 slice command passes. |
| R3 | Save-format churn during Phases 3–5 makes every dev save unloadable | Medium | `schema_version` from commit one; migration functions from commit one; fixture-per-version discipline. Cheap now, impossible to retrofit. |
| R4 | Bevy 0.19 → 0.20 migration mid-Phase 6 | Medium | Bevy only in `presentation` + `desktop` (~15 % of code). Pin exactly; upgrade deliberately between phases, never inside one. |
| R5 | Combat/breeding balance unfalsifiable by inspection | Medium | `tools/balance-simulator` + the 100k-battle nightly with usage-rate assertions. Balance in `config/*.ron`, tunable without recompiling. |
| R6 | Compile times degrade past the point of comfortable iteration | Medium | C5 (editor out of default members), thin crate boundaries, `cargo check -p` during dev, `sccache` in CI. Track `cargo build --timings` at each phase exit. |
| R7 | 1-year offline catch-up exceeds the loading-screen budget on low-end hardware | Low–Medium | D3 budget enforced by bench; 2-year clamp; progress reporting from `offline.rs`. |
| R8 | macOS signing/notarization blocks release | Low, but hard-blocking | Start certificate procurement during Phase 6. |

---

## 8. Immediate next actions

- ~~`git init`; commit `docs/init-plan.md` + this plan as the baseline.~~ Repository initialised on
  `main`; nothing committed yet, so the first commit can be the whole Phase 0 baseline.
- ~~Write ADRs 0006/0007/0008 (D1, D2, D5).~~ Done, plus 0001–0005 and 0009 (events as the only
  outward channel), indexed in [adr/README.md](adr/README.md).
- ~~Execute Phase 0; verify per-crate `clippy.toml` resolution.~~ Done and verified — see the Phase 0
  section above.

Remaining, in order:

1. **Commit and push the current baseline**, so CI proves the Linux and macOS matrix in addition to
   the verified Windows host. Native SQLite/zstd/Bevy builds are the main cross-platform risks.
2. ~~Add the current vertical slice's pet sprite and activity state-machine
   animation.~~ Complete for Emberling: all nine activities and transient event
   reactions are supported. Additional species art belongs with their playable
   presentation slices.
3. ~~Add deterministic rendered-window verification on Windows and Linux.~~
   Implemented and verified locally on Windows; CI now runs the same GPU capture
   under Windows and Linux and uploads both artifacts. Push the baseline to
   obtain the first independent Linux result.
4. ~~Measure and enforce the CPU-side Phase 6 target with eight visible pets.~~
   The 16.67 ms average is regression-gated in the three-OS headless suite, and
   runtime p95 sampling is active. Complete GPU/rendered validation on the
   reference hardware profile as part of action 3.
5. Enable the coverage job in `ci.yml` once its threshold is calibrated against the current core.
