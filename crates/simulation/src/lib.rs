//! Fixed-step, fully deterministic simulation.
//!
//! One tick is one game-minute (`domain::time::TICKS_PER_HOUR == 60`).
//!
//! There is exactly one simulation code path. Offline catch-up is not an
//! approximation of the live loop -- it *is* the live loop, run for the number
//! of elapsed ticks (see ADR 0003 and `offline`). A separate approximate offline
//! path would make `tests/simulation/offline_equivalence.rs` untestable by
//! construction, so none exists.
//!
//! System execution order is a fixed `const` array in `scheduler`, not
//! registration order. Reordering it changes simulation output and is therefore
//! a reviewed, save-schema-relevant change.
//!
//! Performance budget: <= 1 microsecond per pet per tick, enforced by
//! `benches/simulation.rs`. A one-year absence is 525,600 ticks and must catch
//! up inside a loading screen.
//!
//! Phase 3 builds: `config`, `simulation_engine`, `scheduler`, `fixed_step`,
//! `offline`, `systems/*`, `behavior/*`. Phase 5 adds `combat/*`, `breeding/*`.

pub mod breeding;
pub mod combat;
pub mod config;
pub mod fixed_step;
pub mod offline;
pub mod scheduler;
pub mod simulation_engine;
pub mod systems;

pub use config::{ConfigError, SimConfig};
pub use simulation_engine::{SimulationEngine, SimulationState};
