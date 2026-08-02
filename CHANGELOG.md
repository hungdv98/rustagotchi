# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and this project adheres
to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Save compatibility is tracked separately from the version number: any change to a
serialized type bumps `schema_version` and adds a migration plus a fixture. Those
changes are called out under **Saves** below.

## [Unreleased]

### Added — battle progression and adoption economy

- Visible pet level and XP progress on Pet Status and a structured battle-result
  card with winner, rewards, updated wallet, level progress, and level-up feedback.
- Permanent coin-funded adoption-slot upgrades from two founder slots to eight,
  with escalating 100/200/400/800/1,600/3,200 coin prices and affordability copy.

### Saves

- Save schema 5 persists purchased adoption-slot upgrades. Schema 1-4 snapshots
  migrate with the original two-slot capacity.

### Added — desktop pet identity and tactics

- Persistent, validated pet names entered during adoption and displayed on Home,
  Pet Status, breeding, battle, and winner feedback.
- Original transparent Stormling and Aquafin sprites alongside Emberling.
- Power, Precision, and Efficiency battle tactics derived from authored move
  power, accuracy, and energy costs.
- Species-selectable adoption with a visible two-founder capacity and a
  confirmed release flow that protects active battles and pending eggs.
- Confirmation-protected game reset that deletes the active save slot before
  returning to adoption.

### Changed — desktop presentation

- Home is a compact named-pet dashboard with species-specific portraits and
  `Status`, `Breed`, and `Battle` quick actions.
- Pet Status now pairs the selected pet's portrait and identity with its need
  meters; Battle uses opposing fighter cards with HP, energy, moves, and a
  compact tactic bar.
- Main-menu preferences now use stateful `Sound: On/Off` and
  `Motion: Full/Reduced` labels.

### Saves

- Save schema 4 adds the pet name. Schema 1-3 snapshots migrate with the
  `Companion` default, and existing SQLite pet read-model tables gain a `name`
  column automatically.

### Added — Phase 1: the pet aggregate, genetics, inventory and economy

- `domain::pet` completed — the `Pet` aggregate root plus `Stats`, `Personality`, `EmotionalState`
  and `Mood`, `ActivityState`, `HealthState` with ailments, `PetProgression` (integer XP curve),
  `Memories`, `Habits` and `Relationships`. `Pet::set_need` emits a critical-threshold crossing on
  entry only, and `Pet::die` is idempotent, so a per-tick starvation check cannot announce the same
  death several thousand times.
- `domain::genetics` — diploid `GenePair`s with dominance and codominance, `GeneticProfile` with a
  schema check against the species' declared loci, derived `Phenotype`, `MutationRate` in parts per
  million, and `Ancestry` with a relatedness score matching the textbook coefficients (full siblings
  0.5, half siblings 0.25, first cousins 0.125). Nothing anywhere depends on which parent is called
  maternal — asserted exhaustively over every dominance combination.
- `domain::inventory` — `Inventory` (capacity in distinct kinds, all-or-nothing removal),
  `ItemStack` with overflow reporting, `Equipment` that hands back whatever it displaces.
- `domain::economy` — `Wallet` that cannot go negative, `Transaction` whose direction is derived from
  its reason rather than from a sign someone remembered to set.
- `test_support::PetBuilder`, which recomputes derived emotions so a test cannot build a starving pet
  that reports itself delighted.
- Property tests extended to stat bounds, personality blend commutativity, the genetic schema check,
  bounded collections, wallet non-negativity, inventory conservation, and a full `Pet` serde round
  trip.

### Changed

- Removed the `Iterator::sum` ban from `crates/domain/clippy.toml`. It only ever fired on
  provably-bounded sums over primitives: `Milli` and `Fixed` do not implement `Sum`, so the thing it
  was meant to prevent is already impossible by construction. A lint that is always wrong teaches
  people to reach for `#[allow]`.

### Added — Phase 1: domain foundation

- `domain::units` — `Milli` (thousandths of a unit) and `Fixed` (Q16.16). Truncation toward zero,
  saturating arithmetic, `i128` intermediates, and an exact decimal parser/printer for content
  authoring that never touches a float.
- `domain::rng` — `rng_for(seed, stream, tick, entity)` per ADR 0007, over ChaCha8. Unbiased
  `below`, `range_inclusive`, `chance`, `pick` and `shuffle`. Output pinned by a stability test:
  changing it would rewrite the history of every existing world.
- `domain::identifiers` — `ContentKey` with a `snake_case` grammar enforced on construction *and* on
  deserialize; eight content ids; four derived UUIDv8 runtime ids.
- `domain::time` — `Tick`, `GameDuration`, `UnixSeconds` and `GameClock`. The clock carries partial
  ticks forward, so how often the game is polled cannot change how much game time has passed.
- `domain::error`, `domain::validation`, `domain::events` (with `DomainEvents` as a
  `SmallVec<[_; 4]>`), and the `LifeStage` and `PhysicalNeeds` members of the pet aggregate.
- `crates/test-support` — `assert_state_eq` with a structural diff, `NeedsBuilder`, `FakeClock`,
  seeded RNG helpers.
- `tests/` promoted to a package (`rustagotchi-tests`) with explicit `[[test]]` targets, plus the
  Phase 3 determinism and offline-equivalence contracts, written ahead of the code they check.
- 145 tests, including 23 property tests over the range, monotonicity, truncation, clock and
  serde-round-trip invariants.

### Added — Phase 0: foundations

- Cargo workspace with ten library crates and three application binaries,
  `resolver = "3"`, edition 2024, toolchain pinned to 1.97.1.
- Architecture guard (`scripts/check-deps.*`): the dependency closure of
  `domain`, `content`, `simulation` and `application` is snapshot-tested against
  `scripts/dep-allowlist/`, with a denylist backstop for Bevy, SQLite, tokio and
  OS-facing crates.
- Determinism guard: `clippy::disallowed_types` bans `f32`, `f64`, `HashMap`,
  `SystemTime`, `Instant` and `std::path` in `domain` and `simulation`.
- `simulation-cli` command surface: `simulate`, `inspect-save`,
  `validate-content`, `benchmark` (all stubs).
- CI: format, architecture, 3-OS build/clippy/test/release matrix, supply chain
  (`cargo deny`, `cargo audit`). Scheduled security workflow. Dependabot.
- ADRs 0001–0009.
- Local gate scripts (`check`, `check-deps`, `setup`) for bash and PowerShell.

### Changed — corrections to `docs/init-plan.md`

- `ron` 0.11 → 0.12, `rand` 0.9 → 0.10 (with `rand_chacha` 0.10).
- Dropped `panic = "abort"` from the release profile: it would disable the
  `catch_unwind` the crash reporter needs and break `#[should_panic]` under
  `cargo test --release`, benches and fuzzing.
- `rust-version` pinned to the exact toolchain rather than a separate MSRV.
- `apps/dedicated-simulation` deferred post-1.0 (fully overlaps `simulation-cli`).
- `crates/presentation`, `apps/desktop` and `apps/content-editor` excluded from
  the default member set so core work never builds the graphics stack.
- Persistence model changed to snapshot-authoritative with a derived read-model
  (ADR 0008), replacing dual-authoritative tables plus snapshot.

[Unreleased]: https://github.com/rustagotchi/rustagotchi/compare/main...HEAD
