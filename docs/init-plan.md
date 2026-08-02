Below is a production-ready workspace structure for a complex Rust Tamagotchi game with breeding, genetics, combat, offline simulation, persistence, content packs, desktop UI, internal tools, and automated testing.

```text
rustagotchi/
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── rustfmt.toml
├── clippy.toml
├── deny.toml
├── README.md
├── LICENSE
├── CHANGELOG.md
├── CONTRIBUTING.md
├── SECURITY.md
├── .gitignore
├── .editorconfig
├── .env.example
│
├── apps/
│   ├── desktop/
│   │   ├── Cargo.toml
│   │   ├── build.rs
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── app.rs
│   │   │   ├── bootstrap.rs
│   │   │   ├── config.rs
│   │   │   ├── logging.rs
│   │   │   ├── paths.rs
│   │   │   └── plugins/
│   │   │       ├── mod.rs
│   │   │       ├── application_plugin.rs
│   │   │       ├── gameplay_plugin.rs
│   │   │       ├── rendering_plugin.rs
│   │   │       ├── persistence_plugin.rs
│   │   │       └── diagnostics_plugin.rs
│   │   └── resources/
│   │       ├── icons/
│   │       └── packaging/
│   │
│   ├── simulation-cli/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── commands/
│   │       │   ├── mod.rs
│   │       │   ├── simulate.rs
│   │       │   ├── inspect_save.rs
│   │       │   ├── validate_content.rs
│   │       │   └── benchmark.rs
│   │       └── output.rs
│   │
│   ├── content-editor/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── editor.rs
│   │       ├── validation.rs
│   │       ├── preview.rs
│   │       └── panels/
│   │           ├── mod.rs
│   │           ├── species.rs
│   │           ├── items.rs
│   │           ├── moves.rs
│   │           ├── quests.rs
│   │           └── evolution.rs
│   │
│   └── dedicated-simulation/
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── runner.rs
│           └── metrics.rs
│
├── crates/
│   ├── domain/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── error.rs
│   │       ├── identifiers.rs
│   │       ├── validation.rs
│   │       │
│   │       ├── pet/
│   │       │   ├── mod.rs
│   │       │   ├── entity.rs
│   │       │   ├── stats.rs
│   │       │   ├── needs.rs
│   │       │   ├── emotions.rs
│   │       │   ├── mood.rs
│   │       │   ├── personality.rs
│   │       │   ├── activity.rs
│   │       │   ├── life_stage.rs
│   │       │   ├── skills.rs
│   │       │   ├── habits.rs
│   │       │   ├── memories.rs
│   │       │   ├── illnesses.rs
│   │       │   ├── relationships.rs
│   │       │   └── progression.rs
│   │       │
│   │       ├── genetics/
│   │       │   ├── mod.rs
│   │       │   ├── profile.rs
│   │       │   ├── gene.rs
│   │       │   ├── allele.rs
│   │       │   ├── phenotype.rs
│   │       │   ├── mutation.rs
│   │       │   └── lineage.rs
│   │       │
│   │       ├── breeding/
│   │       │   ├── mod.rs
│   │       │   ├── command.rs
│   │       │   ├── compatibility.rs
│   │       │   ├── eligibility.rs
│   │       │   ├── inheritance.rs
│   │       │   ├── offspring.rs
│   │       │   ├── egg.rs
│   │       │   ├── incubation.rs
│   │       │   └── outcome.rs
│   │       │
│   │       ├── combat/
│   │       │   ├── mod.rs
│   │       │   ├── battle.rs
│   │       │   ├── combatant.rs
│   │       │   ├── action.rs
│   │       │   ├── move_definition.rs
│   │       │   ├── move_effect.rs
│   │       │   ├── status_effect.rs
│   │       │   ├── elemental.rs
│   │       │   ├── damage.rs
│   │       │   ├── initiative.rs
│   │       │   ├── rewards.rs
│   │       │   └── result.rs
│   │       │
│   │       ├── inventory/
│   │       │   ├── mod.rs
│   │       │   ├── inventory.rs
│   │       │   ├── item.rs
│   │       │   ├── stack.rs
│   │       │   └── equipment.rs
│   │       │
│   │       ├── world/
│   │       │   ├── mod.rs
│   │       │   ├── world_state.rs
│   │       │   ├── room.rs
│   │       │   ├── furniture.rs
│   │       │   ├── environment.rs
│   │       │   ├── weather.rs
│   │       │   └── location.rs
│   │       │
│   │       ├── economy/
│   │       │   ├── mod.rs
│   │       │   ├── wallet.rs
│   │       │   ├── transaction.rs
│   │       │   ├── shop.rs
│   │       │   └── pricing.rs
│   │       │
│   │       ├── quests/
│   │       │   ├── mod.rs
│   │       │   ├── quest.rs
│   │       │   ├── objective.rs
│   │       │   ├── progress.rs
│   │       │   └── reward.rs
│   │       │
│   │       ├── achievements/
│   │       │   ├── mod.rs
│   │       │   ├── achievement.rs
│   │       │   └── progress.rs
│   │       │
│   │       ├── time/
│   │       │   ├── mod.rs
│   │       │   ├── game_time.rs
│   │       │   ├── duration.rs
│   │       │   └── timestamp.rs
│   │       │
│   │       └── events/
│   │           ├── mod.rs
│   │           ├── domain_event.rs
│   │           ├── pet_event.rs
│   │           ├── breeding_event.rs
│   │           ├── combat_event.rs
│   │           └── world_event.rs
│   │
│   ├── application/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── error.rs
│   │       ├── game_engine.rs
│   │       ├── game_state.rs
│   │       ├── game_session.rs
│   │       ├── service_container.rs
│   │       │
│   │       ├── commands/
│   │       │   ├── mod.rs
│   │       │   ├── pet_command.rs
│   │       │   ├── breeding_command.rs
│   │       │   ├── combat_command.rs
│   │       │   ├── inventory_command.rs
│   │       │   ├── quest_command.rs
│   │       │   └── world_command.rs
│   │       │
│   │       ├── handlers/
│   │       │   ├── mod.rs
│   │       │   ├── pet_handler.rs
│   │       │   ├── breeding_handler.rs
│   │       │   ├── combat_handler.rs
│   │       │   ├── inventory_handler.rs
│   │       │   ├── quest_handler.rs
│   │       │   └── world_handler.rs
│   │       │
│   │       ├── queries/
│   │       │   ├── mod.rs
│   │       │   ├── pet_query.rs
│   │       │   ├── combat_query.rs
│   │       │   ├── lineage_query.rs
│   │       │   └── inventory_query.rs
│   │       │
│   │       ├── dto/
│   │       │   ├── mod.rs
│   │       │   ├── pet_dto.rs
│   │       │   ├── battle_dto.rs
│   │       │   ├── breeding_dto.rs
│   │       │   └── inventory_dto.rs
│   │       │
│   │       └── ports/
│   │           ├── mod.rs
│   │           ├── save_repository.rs
│   │           ├── content_repository.rs
│   │           ├── clock.rs
│   │           ├── random_source.rs
│   │           ├── notification_service.rs
│   │           └── telemetry.rs
│   │
│   ├── simulation/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── config.rs
│   │       ├── simulation_engine.rs
│   │       ├── scheduler.rs
│   │       ├── fixed_step.rs
│   │       ├── offline.rs
│   │       │
│   │       ├── systems/
│   │       │   ├── mod.rs
│   │       │   ├── needs_decay.rs
│   │       │   ├── emotional_update.rs
│   │       │   ├── health_update.rs
│   │       │   ├── illness_update.rs
│   │       │   ├── aging.rs
│   │       │   ├── growth.rs
│   │       │   ├── evolution.rs
│   │       │   ├── egg_incubation.rs
│   │       │   ├── cooldowns.rs
│   │       │   ├── habits.rs
│   │       │   ├── memory_decay.rs
│   │       │   ├── quest_progress.rs
│   │       │   └── environment.rs
│   │       │
│   │       ├── behavior/
│   │       │   ├── mod.rs
│   │       │   ├── utility_ai.rs
│   │       │   ├── behavior.rs
│   │       │   ├── scorer.rs
│   │       │   ├── decision.rs
│   │       │   └── actions/
│   │       │       ├── mod.rs
│   │       │       ├── eat.rs
│   │       │       ├── sleep.rs
│   │       │       ├── play.rs
│   │       │       ├── explore.rs
│   │       │       ├── socialize.rs
│   │       │       └── clean.rs
│   │       │
│   │       ├── combat/
│   │       │   ├── mod.rs
│   │       │   ├── battle_engine.rs
│   │       │   ├── turn_resolver.rs
│   │       │   ├── action_resolver.rs
│   │       │   ├── combat_ai.rs
│   │       │   ├── status_processor.rs
│   │       │   └── reward_calculator.rs
│   │       │
│   │       └── breeding/
│   │           ├── mod.rs
│   │           ├── breeding_engine.rs
│   │           ├── compatibility_calculator.rs
│   │           ├── gene_inheritance.rs
│   │           ├── mutation_engine.rs
│   │           └── species_resolver.rs
│   │
│   ├── content/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── error.rs
│   │       ├── manifest.rs
│   │       ├── loader.rs
│   │       ├── registry.rs
│   │       ├── validator.rs
│   │       ├── cache.rs
│   │       │
│   │       ├── definitions/
│   │       │   ├── mod.rs
│   │       │   ├── species.rs
│   │       │   ├── evolution.rs
│   │       │   ├── item.rs
│   │       │   ├── food.rs
│   │       │   ├── combat_move.rs
│   │       │   ├── illness.rs
│   │       │   ├── quest.rs
│   │       │   ├── room.rs
│   │       │   ├── furniture.rs
│   │       │   └── achievement.rs
│   │       │
│   │       ├── formats/
│   │       │   ├── mod.rs
│   │       │   ├── ron.rs
│   │       │   └── json.rs
│   │       │
│   │       └── localization/
│   │           ├── mod.rs
│   │           ├── locale.rs
│   │           └── translator.rs
│   │
│   ├── persistence/
│   │   ├── Cargo.toml
│   │   ├── migrations/
│   │   │   ├── 0001_initial.sql
│   │   │   ├── 0002_add_genetics.sql
│   │   │   ├── 0003_add_battles.sql
│   │   │   └── 0004_add_lineage.sql
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── error.rs
│   │       ├── connection.rs
│   │       ├── transaction.rs
│   │       ├── migration.rs
│   │       ├── backup.rs
│   │       ├── recovery.rs
│   │       ├── validation.rs
│   │       │
│   │       ├── repositories/
│   │       │   ├── mod.rs
│   │       │   ├── game_repository.rs
│   │       │   ├── pet_repository.rs
│   │       │   ├── lineage_repository.rs
│   │       │   ├── inventory_repository.rs
│   │       │   ├── quest_repository.rs
│   │       │   └── battle_repository.rs
│   │       │
│   │       ├── models/
│   │       │   ├── mod.rs
│   │       │   ├── game_model.rs
│   │       │   ├── pet_model.rs
│   │       │   ├── genetics_model.rs
│   │       │   ├── battle_model.rs
│   │       │   ├── lineage_model.rs
│   │       │   └── inventory_model.rs
│   │       │
│   │       └── serialization/
│   │           ├── mod.rs
│   │           ├── save_envelope.rs
│   │           ├── save_version.rs
│   │           └── checksum.rs
│   │
│   ├── platform/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── paths.rs
│   │       ├── clock.rs
│   │       ├── random.rs
│   │       ├── notifications.rs
│   │       ├── file_lock.rs
│   │       ├── process_lock.rs
│   │       └── lifecycle.rs
│   │
│   ├── presentation/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── app_state.rs
│   │       ├── resources.rs
│   │       ├── events.rs
│   │       ├── view_models.rs
│   │       │
│   │       ├── plugins/
│   │       │   ├── mod.rs
│   │       │   ├── ui_plugin.rs
│   │       │   ├── pet_plugin.rs
│   │       │   ├── combat_plugin.rs
│   │       │   ├── breeding_plugin.rs
│   │       │   ├── animation_plugin.rs
│   │       │   ├── audio_plugin.rs
│   │       │   └── scene_plugin.rs
│   │       │
│   │       ├── screens/
│   │       │   ├── mod.rs
│   │       │   ├── loading.rs
│   │       │   ├── main_menu.rs
│   │       │   ├── pet_creation.rs
│   │       │   ├── home.rs
│   │       │   ├── pet_status.rs
│   │       │   ├── inventory.rs
│   │       │   ├── breeding.rs
│   │       │   ├── lineage.rs
│   │       │   ├── combat_lobby.rs
│   │       │   ├── battle.rs
│   │       │   ├── shop.rs
│   │       │   ├── quests.rs
│   │       │   ├── settings.rs
│   │       │   └── save_slots.rs
│   │       │
│   │       ├── widgets/
│   │       │   ├── mod.rs
│   │       │   ├── stat_bar.rs
│   │       │   ├── pet_card.rs
│   │       │   ├── item_slot.rs
│   │       │   ├── move_button.rs
│   │       │   ├── dialogue_box.rs
│   │       │   ├── notification.rs
│   │       │   └── modal.rs
│   │       │
│   │       ├── animation/
│   │       │   ├── mod.rs
│   │       │   ├── controller.rs
│   │       │   ├── state_machine.rs
│   │       │   └── sprite_sheet.rs
│   │       │
│   │       └── audio/
│   │           ├── mod.rs
│   │           ├── mixer.rs
│   │           ├── music.rs
│   │           └── sound_effects.rs
│   │
│   ├── minigames/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── minigame.rs
│   │       ├── registry.rs
│   │       ├── result.rs
│   │       ├── memory_match/
│   │       │   ├── mod.rs
│   │       │   ├── state.rs
│   │       │   └── systems.rs
│   │       ├── obstacle_run/
│   │       │   ├── mod.rs
│   │       │   ├── state.rs
│   │       │   └── systems.rs
│   │       └── food_catch/
│   │           ├── mod.rs
│   │           ├── state.rs
│   │           └── systems.rs
│   │
│   ├── telemetry/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── logging.rs
│   │       ├── metrics.rs
│   │       ├── performance.rs
│   │       └── crash_report.rs
│   │
│   └── test-support/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── builders.rs
│           ├── fixtures.rs
│           ├── fake_clock.rs
│           ├── seeded_rng.rs
│           ├── in_memory_repository.rs
│           └── assertions.rs
│
├── assets/
│   ├── manifest.ron
│   ├── content/
│   │   ├── species/
│   │   │   ├── emberling.ron
│   │   │   ├── stormling.ron
│   │   │   └── aquafin.ron
│   │   ├── evolution/
│   │   │   ├── emberling_tree.ron
│   │   │   └── stormling_tree.ron
│   │   ├── breeding/
│   │   │   ├── compatibility.ron
│   │   │   └── hybrid_species.ron
│   │   ├── combat/
│   │   │   ├── moves/
│   │   │   ├── statuses/
│   │   │   ├── elements.ron
│   │   │   └── balance.ron
│   │   ├── items/
│   │   │   ├── food/
│   │   │   ├── medicine/
│   │   │   ├── toys/
│   │   │   └── equipment/
│   │   ├── quests/
│   │   ├── achievements/
│   │   ├── rooms/
│   │   ├── furniture/
│   │   └── illnesses/
│   │
│   ├── graphics/
│   │   ├── pets/
│   │   │   ├── emberling/
│   │   │   │   ├── idle.png
│   │   │   │   ├── walk.png
│   │   │   │   ├── eat.png
│   │   │   │   ├── sleep.png
│   │   │   │   ├── attack.png
│   │   │   │   └── hurt.png
│   │   │   └── stormling/
│   │   ├── rooms/
│   │   ├── furniture/
│   │   ├── items/
│   │   ├── effects/
│   │   └── ui/
│   │
│   ├── audio/
│   │   ├── music/
│   │   ├── ambience/
│   │   ├── interface/
│   │   ├── pet/
│   │   └── combat/
│   │
│   ├── fonts/
│   ├── localization/
│   │   ├── en-US.ftl
│   │   ├── vi-VN.ftl
│   │   └── ja-JP.ftl
│   └── shaders/
│
├── config/
│   ├── default.ron
│   ├── development.ron
│   ├── production.ron
│   ├── simulation.ron
│   ├── combat_balance.ron
│   └── breeding_balance.ron
│
├── tests/
│   ├── integration/
│   │   ├── game_lifecycle.rs
│   │   ├── pet_care_flow.rs
│   │   ├── breeding_flow.rs
│   │   ├── egg_hatching_flow.rs
│   │   ├── battle_flow.rs
│   │   ├── save_load_flow.rs
│   │   ├── offline_progression.rs
│   │   └── evolution_flow.rs
│   │
│   ├── simulation/
│   │   ├── seven_day_simulation.rs
│   │   ├── one_year_simulation.rs
│   │   ├── deterministic_simulation.rs
│   │   └── offline_equivalence.rs
│   │
│   ├── content/
│   │   ├── validate_all_content.rs
│   │   ├── validate_evolution_graph.rs
│   │   ├── validate_move_references.rs
│   │   └── validate_localization.rs
│   │
│   └── persistence/
│       ├── migration_compatibility.rs
│       ├── corrupted_save_recovery.rs
│       ├── atomic_save.rs
│       └── concurrent_save_lock.rs
│
├── benches/
│   ├── simulation.rs
│   ├── offline_progression.rs
│   ├── combat_resolution.rs
│   ├── breeding_generation.rs
│   └── save_load.rs
│
├── fuzz/
│   ├── Cargo.toml
│   └── fuzz_targets/
│       ├── load_save.rs
│       ├── parse_content.rs
│       ├── resolve_battle.rs
│       └── generate_offspring.rs
│
├── tools/
│   ├── content-validator/
│   ├── sprite-packer/
│   ├── save-migrator/
│   ├── balance-simulator/
│   └── release-builder/
│
├── scripts/
│   ├── setup.sh
│   ├── setup.ps1
│   ├── check.sh
│   ├── test.sh
│   ├── package.sh
│   ├── validate-content.sh
│   └── generate-release-notes.sh
│
├── packaging/
│   ├── windows/
│   │   ├── wix/
│   │   └── installer-assets/
│   ├── linux/
│   │   ├── appimage/
│   │   ├── deb/
│   │   └── desktop-entry/
│   └── macos/
│       ├── entitlements.plist
│       └── bundle/
│
├── docs/
│   ├── 00-product-vision.md
│   ├── 01-game-design-document.md
│   ├── 02-architecture.md
│   ├── 03-domain-model.md
│   ├── 04-simulation-design.md
│   ├── 05-breeding-and-genetics.md
│   ├── 06-combat-system.md
│   ├── 07-content-authoring.md
│   ├── 08-persistence.md
│   ├── 09-save-migrations.md
│   ├── 10-testing-strategy.md
│   ├── 11-balancing-guide.md
│   ├── 12-release-process.md
│   ├── adr/
│   │   ├── 0001-use-workspace.md
│   │   ├── 0002-domain-independent-from-bevy.md
│   │   ├── 0003-use-fixed-step-simulation.md
│   │   ├── 0004-use-sqlite.md
│   │   └── 0005-data-driven-content.md
│   └── diagrams/
│       ├── system-context.md
│       ├── component-architecture.md
│       ├── battle-sequence.md
│       ├── breeding-sequence.md
│       └── save-lifecycle.md
│
└── .github/
    ├── workflows/
    │   ├── ci.yml
    │   ├── security.yml
    │   ├── content-validation.yml
    │   ├── release.yml
    │   └── nightly-simulation.yml
    ├── ISSUE_TEMPLATE/
    ├── pull_request_template.md
    └── dependabot.yml
```

## Root workspace configuration

```toml
[workspace]
resolver = "3"
members = [
    "apps/desktop",
    "apps/simulation-cli",
    "apps/content-editor",
    "apps/dedicated-simulation",

    "crates/domain",
    "crates/application",
    "crates/simulation",
    "crates/content",
    "crates/persistence",
    "crates/platform",
    "crates/presentation",
    "crates/minigames",
    "crates/telemetry",
    "crates/test-support",
]

default-members = [
    "apps/desktop",
]

[workspace.package]
version = "0.1.0"
edition = "2024"
license = "MIT"
rust-version = "1.85"
authors = ["Rustagotchi Team"]

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
ron = "0.11"
thiserror = "2"
anyhow = "1"
tracing = "0.1"
uuid = { version = "1", features = ["v4", "serde"] }
rand = "0.9"
chrono = { version = "0.4", features = ["serde"] }

[profile.dev]
opt-level = 1

[profile.dev.package."*"]
opt-level = 3

[profile.release]
lto = "thin"
codegen-units = 1
strip = true
panic = "abort"

[profile.profiling]
inherits = "release"
debug = true
strip = false
```

# Dependency direction

The dependency graph must remain one-way:

```text
presentation ───────┐
                    ▼
desktop ─────► application ─────► domain
                    │               ▲
                    ├──► simulation ┘
                    ├──► content
                    └──► persistence

persistence ─────► application ports
platform ────────► application ports
minigames ───────► domain
```

The critical rule is:

```text
domain must not depend on:
- Bevy
- SQLite
- file systems
- operating-system APIs
- UI code
- application configuration
```

# Responsibility of each major crate

## `domain`

Contains pure business rules:

* Pet state
* Genetics
* Breeding eligibility
* Combat entities
* Inventory
* Quests
* Evolution
* Value objects
* Domain events

No rendering or database dependencies.

## `application`

Coordinates use cases:

* Feed a pet
* Start breeding
* Create an egg
* Start a battle
* Execute a combat turn
* Save a session
* Load a game
* Complete a quest

This crate exposes commands, queries, handlers, and repository interfaces.

## `simulation`

Runs time-dependent behavior:

* Hunger decay
* Mood updates
* Illness progression
* Aging
* Offline progression
* Egg incubation
* AI behavior
* Combat resolution
* Genetic inheritance

## `content`

Loads and validates data-driven definitions:

* Species
* Items
* Combat moves
* Breeding rules
* Evolution trees
* Quests
* Rooms
* Localization

## `persistence`

Implements application repository interfaces using SQLite and serialized snapshots.

## `presentation`

Contains Bevy scenes, UI systems, animations, sound, input handling, and view models.

## `platform`

Implements OS-dependent capabilities:

* Save directories
* Desktop notifications
* File locking
* Real clock
* Secure random source
* Application lifecycle

# Recommended internal domain boundaries

The main aggregate roots should be:

```rust
pub struct GameState {
    pub player: PlayerProfile,
    pub pets: PetCollection,
    pub eggs: EggCollection,
    pub inventory: Inventory,
    pub world: WorldState,
    pub quests: QuestProgress,
    pub economy: EconomyState,
    pub clock: GameClock,
    pub metadata: SaveMetadata,
}
```

```rust
pub struct Pet {
    pub identity: PetIdentity,
    pub species: SpeciesId,
    pub stage: LifeStage,
    pub needs: PhysicalNeeds,
    pub emotions: EmotionalState,
    pub personality: Personality,
    pub genetics: GeneticProfile,
    pub progression: PetProgression,
    pub breeding: BreedingState,
    pub combat: CombatProfile,
    pub health: HealthState,
    pub memories: Vec<Memory>,
    pub habits: Vec<Habit>,
}
```

```rust
pub struct Battle {
    pub id: BattleId,
    pub first: Combatant,
    pub second: Combatant,
    pub turn: u32,
    pub phase: BattlePhase,
    pub events: Vec<BattleEvent>,
    pub result: Option<BattleResult>,
}
```

```rust
pub struct Egg {
    pub id: EggId,
    pub parent_ids: [PetId; 2],
    pub species_seed: SpeciesId,
    pub genetics: GeneticProfile,
    pub incubation: IncubationState,
}
```

# Runtime flow

```text
Player input
    ↓
Bevy presentation system
    ↓
Application command
    ↓
Command handler
    ↓
Domain validation
    ↓
Simulation or domain operation
    ↓
Domain events
    ↓
Update GameState
    ↓
Persist state
    ↓
Convert events to animation, audio and UI feedback
```

Example breeding flow:

```text
Breeding screen
    ↓
StartBreeding command
    ↓
Eligibility validation
    ↓
Lineage validation
    ↓
Compatibility calculation
    ↓
Gene inheritance
    ↓
Mutation evaluation
    ↓
Egg created
    ↓
Parents receive cooldown
    ↓
Save transaction
    ↓
Egg-created animation
```

Example battle flow:

```text
Battle action selected
    ↓
SubmitBattleAction command
    ↓
Validate move and energy
    ↓
Resolve initiative
    ↓
Calculate hit, damage and effects
    ↓
Emit BattleEvents
    ↓
Update battle state
    ↓
Render animation and combat log
```

# Persistence model

Use both normalized tables and a versioned save snapshot.

Recommended SQLite tables:

```sql
save_slots
game_snapshots
pets
pet_genetics
pet_memories
pet_habits
pet_illnesses
pet_relationships
eggs
lineage
inventory
quest_progress
achievements
battle_history
world_state
schema_migrations
```

A save operation should be transactional:

```text
Acquire save lock
    ↓
Begin SQLite transaction
    ↓
Validate GameState
    ↓
Write normalized state
    ↓
Write versioned snapshot
    ↓
Write checksum
    ↓
Commit
    ↓
Create rotating backup
    ↓
Release lock
```

# Production CI checks

Every pull request should run:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo deny check
cargo audit
cargo build --workspace --release
cargo run -p simulation-cli -- validate-content
```

Nightly checks should run:

```text
7-day simulated pet lifecycle
1-year accelerated lifecycle
100,000 generated offspring
100,000 automated battles
save/load equivalence
offline/live simulation equivalence
evolution graph reachability
content-reference validation
```

# Recommended implementation order

Start only with these crates:

```text
crates/domain
crates/application
crates/simulation
crates/content
crates/persistence
apps/simulation-cli
```

Build a complete headless vertical slice:

```text
Create pet
Feed pet
Advance time
Breed two pets
Generate egg
Hatch offspring
Start battle
Resolve battle
Save game
Load game
```

After this works reliably, add:

```text
crates/presentation
crates/platform
apps/desktop
```

Then add minigames, content editor, packaging, localization, and advanced telemetry.

This structure is large enough for a production game while keeping the core pet, breeding, and combat logic independently testable and reusable across desktop, mobile, web, or future multiplayer clients.
