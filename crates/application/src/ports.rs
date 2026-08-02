use domain::events::DomainEvent;
use domain::identifiers::SaveSlotId;
use domain::rng::WorldSeed;
use domain::time::UnixSeconds;

use crate::{GameState, SaveError, SaveMetadata, SeedError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotSummary {
    pub slot: SaveSlotId,
    pub schema_version: u32,
    pub updated_at: UnixSeconds,
}

pub trait Clock: Send + Sync {
    fn now_unix(&self) -> UnixSeconds;
}
pub trait RandomSource: Send + Sync {
    fn world_seed(&self) -> Result<WorldSeed, SeedError>;
}
pub trait ContentRepository: Send + Sync {
    fn load(&self) -> Result<content::ContentRegistry, content::ContentError>;
}
pub trait SaveRepository: Send + Sync {
    fn list_slots(&self) -> Result<Vec<SlotSummary>, SaveError>;
    fn save(&self, slot: SaveSlotId, state: &GameState) -> Result<SaveMetadata, SaveError>;
    fn load(&self, slot: SaveSlotId) -> Result<GameState, SaveError>;
    fn delete(&self, slot: SaveSlotId) -> Result<(), SaveError>;
}

impl<T: SaveRepository + ?Sized> SaveRepository for Box<T> {
    fn list_slots(&self) -> Result<Vec<SlotSummary>, SaveError> {
        (**self).list_slots()
    }

    fn save(&self, slot: SaveSlotId, state: &GameState) -> Result<SaveMetadata, SaveError> {
        (**self).save(slot, state)
    }

    fn load(&self, slot: SaveSlotId) -> Result<GameState, SaveError> {
        (**self).load(slot)
    }

    fn delete(&self, slot: SaveSlotId) -> Result<(), SaveError> {
        (**self).delete(slot)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notification {
    pub title: String,
    pub body: String,
}

pub trait NotificationService: Send + Sync {
    fn notify(&self, notification: Notification);
}

pub trait Telemetry: Send + Sync {
    fn record(&self, event: &DomainEvent);
}
