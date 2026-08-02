//! Headless driver for the Rustagotchi core.
//!
//! This binary is how the game is developed and verified through Phase 5: the
//! entire vertical slice (create -> feed -> advance -> breed -> hatch -> battle
//! -> save -> load) runs here, with no pixels involved.
//!
//! Phase 0 defines the command surface; each subcommand is implemented in the
//! phase named in its help text.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

mod benchmark;
mod scenario;

#[derive(Debug, Parser)]
#[command(name = "simulation-cli", version, about, long_about = None)]
struct Cli {
    /// Increase log verbosity. Repeat for more detail.
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run the deterministic simulation headlessly.
    Simulate {
        /// Scenario script to execute, e.g. tests/scripts/vertical_slice.ron.
        #[arg(long)]
        script: Option<PathBuf>,

        /// Number of ticks to advance. One tick is one game-minute.
        #[arg(long, default_value_t = 1_440)]
        ticks: u64,

        /// World seed. Fixed by default so runs are reproducible.
        #[arg(long, default_value_t = 0)]
        seed: u64,

        /// Content and simulation-config root.
        #[arg(long, default_value = "assets")]
        assets: PathBuf,

        /// Persist the resulting world to this SQLite database.
        #[arg(long)]
        save: Option<PathBuf>,
    },

    /// Decode a save file and print its envelope, schema version and contents. [Phase 4]
    InspectSave {
        /// Path to the save database.
        path: PathBuf,

        /// Emit JSON instead of a human-readable summary.
        #[arg(long)]
        json: bool,
    },

    /// Load every content pack and run the full validator. [Phase 2]
    ValidateContent {
        /// Content root. Defaults to ./assets.
        #[arg(long, default_value = "assets")]
        assets: PathBuf,
    },

    /// Run scale checks: mass offspring generation, mass battles, timing. [Phase 5]
    Benchmark {
        /// Number of offspring to generate.
        #[arg(long)]
        offspring: Option<u64>,

        /// Number of battles to resolve.
        #[arg(long)]
        battles: Option<u64>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    init_tracing(cli.verbose);

    if let Command::ValidateContent { assets } = &cli.command {
        return match content::ContentLoader::load(assets) {
            Ok(registry) => {
                if let Err(error) = load_sim_config(assets) {
                    eprintln!("simulation configuration failed validation:\n{error}");
                    return ExitCode::from(1);
                }
                print_content_summary(&registry);
                println!("simulation config       valid");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("content validation failed:\n{error}");
                ExitCode::from(1)
            }
        };
    }
    if let Command::Simulate {
        script,
        ticks,
        seed,
        assets,
        save,
    } = &cli.command
    {
        if let Some(path) = script {
            return match scenario::run(path, assets) {
                Ok(summary) => {
                    println!("{summary}");
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("vertical slice failed: {error}");
                    ExitCode::from(1)
                }
            };
        }
        return run_simulation(*seed, *ticks, assets, save.as_deref());
    }
    if let Command::InspectSave { path, json } = &cli.command {
        return inspect_save(path, *json);
    }
    if let Command::Benchmark { offspring, battles } = &cli.command {
        return match benchmark::run(*offspring, *battles, std::path::Path::new("assets")) {
            Ok(summary) => {
                println!("{summary}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("benchmark failed: {error}");
                ExitCode::from(1)
            }
        };
    }

    let (name, phase) = match cli.command {
        Command::Simulate { .. } => ("simulate", "Phase 3"),
        Command::InspectSave { .. } => ("inspect-save", "Phase 4"),
        Command::ValidateContent { .. } => ("validate-content", "Phase 2"),
        Command::Benchmark { .. } => ("benchmark", "Phase 5"),
    };

    eprintln!("simulation-cli: `{name}` is not implemented yet (scheduled for {phase}).");
    eprintln!("See docs/implementation-plan.md for the phase plan.");
    ExitCode::from(2)
}

fn inspect_save(path: &std::path::Path, json: bool) -> ExitCode {
    use application::ports::SaveRepository;
    let repository = match persistence::SqliteSaveRepository::new(path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("cannot open save: {error}");
            return ExitCode::from(1);
        }
    };
    let slots = match repository.list_slots() {
        Ok(value) => value,
        Err(error) => {
            eprintln!("cannot list save slots: {error}");
            return ExitCode::from(1);
        }
    };
    if slots.is_empty() {
        eprintln!("save database contains no slots");
        return ExitCode::from(1);
    }
    for summary in slots {
        let state = match repository.load(summary.slot) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("slot {} failed: {error}", summary.slot);
                continue;
            }
        };
        if json {
            match serde_json::to_string_pretty(&state) {
                Ok(value) => println!("{value}"),
                Err(error) => {
                    eprintln!("cannot encode slot {}: {error}", summary.slot);
                    return ExitCode::from(1);
                }
            }
        } else {
            println!(
                "slot {}: schema={}, saved={}, tick={}, pets={}",
                summary.slot,
                summary.schema_version,
                summary.updated_at.0,
                state.simulation.tick.0,
                state.simulation.pets.len()
            );
        }
    }
    ExitCode::SUCCESS
}

fn run_simulation(
    seed: u64,
    ticks: u64,
    assets: &std::path::Path,
    save: Option<&std::path::Path>,
) -> ExitCode {
    use domain::genetics::{Ancestry, GeneticProfile, Parentage};
    use domain::identifiers::{PetId, SaveSlotId, SpeciesId};
    use domain::pet::{Personality, Pet};
    use domain::rng::WorldSeed;
    use domain::time::{GameClock, GameDuration, Tick, UnixSeconds};

    let content = match content::ContentLoader::load(assets) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("content loading failed:\n{error}");
            return ExitCode::from(1);
        }
    };
    let config = match load_sim_config(assets) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    let seed = WorldSeed(seed);
    let mut state = simulation::SimulationState::new(seed);
    let id = PetId::derive(seed, Tick::ZERO, 1);
    state.insert_pet(Pet::hatch(
        id,
        SpeciesId::new("emberling").expect("base content id"),
        GeneticProfile::new(),
        Personality::neutral(),
        Parentage::Founder,
        Ancestry::founder(),
        Tick::ZERO,
    ));
    let mut engine = simulation::SimulationEngine::new(state, config);
    let events = engine.advance(GameDuration::from_ticks(ticks), &content);
    let simulation = engine.into_state();
    let pet = simulation.pet(id).expect("inserted pet");
    println!("simulated {ticks} ticks with seed {}", seed.0);
    println!(
        "pet: species={}, stage={}, level={}, alive={}, health={}, events={}",
        pet.species,
        pet.stage,
        pet.progression.level().get(),
        pet.is_alive(),
        pet.health.current(),
        events.len()
    );
    if let Some(path) = save {
        use application::ports::SaveRepository;
        let mut game = application::GameState::new(seed, UnixSeconds(0));
        game.simulation = simulation;
        game.clock = GameClock::starting_at(UnixSeconds(0));
        game.clock.advance(GameDuration::from_ticks(ticks));
        let repository = match persistence::SqliteSaveRepository::new(path) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("cannot create save: {error}");
                return ExitCode::from(1);
            }
        };
        let slot = SaveSlotId::derive(seed, Tick::ZERO, 1);
        if let Err(error) = repository.save(slot, &game) {
            eprintln!("cannot save simulation: {error}");
            return ExitCode::from(1);
        }
        println!("saved slot {slot} to {}", path.display());
    }
    ExitCode::SUCCESS
}

fn load_sim_config(assets: &std::path::Path) -> Result<simulation::SimConfig, String> {
    let path = assets.join("config/simulation.ron");
    let source = std::fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    simulation::SimConfig::from_ron(&source).map_err(|error| format!("{}: {error}", path.display()))
}

fn print_content_summary(registry: &content::ContentRegistry) {
    let counts = registry.counts();
    println!("Content pack `{}` is valid.", registry.pack_id());
    println!("{:<24} {:>6}", "kind", "count");
    println!("{:-<31}", "");
    for (kind, count) in [
        ("species", counts.species),
        ("evolutions", counts.evolutions),
        ("items", counts.items),
        ("moves", counts.moves),
        ("illnesses", counts.illnesses),
        ("compatibility entries", counts.compatibility_entries),
        ("en-US translations", counts.translations),
    ] {
        println!("{kind:<24} {count:>6}");
    }
}

fn init_tracing(verbosity: u8) {
    let default = match verbosity {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(default));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();
}
