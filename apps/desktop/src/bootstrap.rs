use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use application::ports::{Clock, RandomSource, SaveRepository};
use application::{GameEngine, GameState};
use bevy::prelude::Resource;
use content::ContentLoader;
use domain::identifiers::SaveSlotId;
use domain::time::Tick;
use persistence::SqliteSaveRepository;
use platform::{AppPaths, ProcessLock, SystemClock, SystemRandomSource};
use presentation::{ActiveSaveSlot, EngineResource};

#[derive(Debug, Resource)]
pub(crate) struct DesktopProcessLock {
    _lock: ProcessLock,
}

#[derive(Debug)]
pub(crate) struct Bootstrap {
    pub(crate) engine: EngineResource,
    pub(crate) slot: ActiveSaveSlot,
    pub(crate) assets: PathBuf,
    pub(crate) process_lock: DesktopProcessLock,
    pub(crate) startup_events: Vec<domain::events::DomainEvent>,
}

impl Bootstrap {
    pub(crate) fn load() -> Result<Self> {
        let paths = AppPaths::discover().context("locating the application data directory")?;
        paths.ensure().context("creating application directories")?;
        let process_lock = ProcessLock::acquire(&paths)
            .context("another Rustagotchi process is already using this profile")?;

        let assets = discover_assets()?;
        let content = ContentLoader::load(&assets).context("loading game content")?;
        let config_source = std::fs::read_to_string(assets.join("config/simulation.ron"))
            .context("reading simulation configuration")?;
        let config = simulation::SimConfig::from_ron(&config_source)
            .context("validating simulation configuration")?;

        let repository = SqliteSaveRepository::new(paths.saves.join("rustagotchi.sqlite3"))
            .context("opening the save database")?;
        let now = SystemClock.now_unix();
        let slots = repository.list_slots().context("listing save slots")?;
        let (slot, state) = match slots.first() {
            Some(existing) => (
                existing.slot,
                repository
                    .load(existing.slot)
                    .context("loading the most recent save")?,
            ),
            None => {
                let seed = SystemRandomSource
                    .world_seed()
                    .context("generating a new world seed")?;
                (
                    SaveSlotId::derive(seed, Tick::ZERO, 0),
                    GameState::new(seed, now),
                )
            }
        };

        let saves: Box<dyn SaveRepository> = Box::new(repository);
        let mut engine = GameEngine::new(state, content, config, saves);
        let startup_events = engine
            .advance_to(now)
            .context("applying offline progression")?;
        tracing::info!(
            slot = %slot,
            offline_events = startup_events.len(),
            assets = %assets.display(),
            "desktop world initialized"
        );

        Ok(Self {
            engine: EngineResource(engine),
            slot: ActiveSaveSlot(slot),
            assets,
            process_lock: DesktopProcessLock {
                _lock: process_lock,
            },
            startup_events,
        })
    }
}

fn discover_assets() -> Result<PathBuf> {
    if let Some(configured) = std::env::var_os("RUSTAGOTCHI_ASSETS") {
        return validate_assets(PathBuf::from(configured));
    }

    let mut candidates = Vec::new();
    if let Ok(current) = std::env::current_dir() {
        candidates.push(current.join("assets"));
    }
    if let Ok(executable) = std::env::current_exe()
        && let Some(parent) = executable.parent()
    {
        candidates.push(parent.join("assets"));
    }

    for candidate in candidates {
        if candidate.join("manifest.ron").is_file() {
            return Ok(candidate);
        }
    }
    bail!("cannot find assets; set RUSTAGOTCHI_ASSETS to the content directory")
}

fn validate_assets(path: PathBuf) -> Result<PathBuf> {
    if !Path::new(&path).join("manifest.ron").is_file() {
        bail!("{} does not contain manifest.ron", path.display());
    }
    Ok(path)
}
