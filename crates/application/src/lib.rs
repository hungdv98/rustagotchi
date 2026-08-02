//! Use-case coordination: the only surface an application binary talks to.
//!
//! `ports` declares the traits the outside world implements (`Clock`,
//! `SaveRepository`, `ContentRepository`, `NotificationService`, `Telemetry`).
//! `persistence` and `platform` depend on this crate to implement them -- the
//! arrow points inward, which is what keeps `domain` free of I/O.
//!
//! `dto` exists so that `presentation` never sees a domain type. That boundary
//! is what stops Bevy concerns leaking backwards into the core when Phase 6
//! lands.
//!
//! Every operation has the shape
//! `fn(..) -> Result<Vec<DomainEvent>, AppError>` (ADR 0009): events are the
//! only channel to presentation, telemetry and quest progress.
//!
//! Phase 4 builds: `game_engine`, `game_state`, `game_session`,
//! `service_container`, `commands/*`, `handlers/*`, `queries/*`, `dto/*`,
//! `ports/*`.

pub mod dto;
pub mod error;
pub mod game_engine;
pub mod game_state;
pub mod ports;
pub mod queries;

pub use error::{AppError, SaveError, SeedError};
pub use game_engine::{
    BASE_ADOPTED_PETS, Command, GameEngine, MAX_ADOPTED_PETS, MAX_ADOPTION_SLOT_UPGRADES,
    adopted_pet_capacity, next_adoption_slot_cost,
};
pub use game_state::{GameState, SaveMetadata};
pub use queries::{BattleSetupQuery, ContentQuery, GameSummaryQuery, Query};
