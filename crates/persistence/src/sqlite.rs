use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};

use application::ports::{SaveRepository, SlotSummary};
use application::{GameState, SaveError, SaveMetadata};
use chrono::Utc;
use domain::identifiers::SaveSlotId;
use domain::time::UnixSeconds;
use fs4::FileExt;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

use crate::SaveEnvelope;

#[derive(Debug, Clone)]
pub struct SqliteSaveRepository {
    path: PathBuf,
}

impl SqliteSaveRepository {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, SaveError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(storage)?;
        }
        let repository = Self { path };
        repository.initialize(&repository.path)?;
        Ok(repository)
    }
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    fn initialize(&self, path: &Path) -> Result<(), SaveError> {
        let connection = Connection::open(path).map_err(storage)?;
        connection.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=DELETE;
            CREATE TABLE IF NOT EXISTS save_slots(slot BLOB PRIMARY KEY, schema_version INTEGER NOT NULL, updated_at INTEGER NOT NULL);
            CREATE TABLE IF NOT EXISTS game_snapshots(slot BLOB PRIMARY KEY REFERENCES save_slots(slot) ON DELETE CASCADE, envelope BLOB NOT NULL);
            CREATE TABLE IF NOT EXISTS pets(slot BLOB NOT NULL REFERENCES save_slots(slot) ON DELETE CASCADE, pet_id TEXT NOT NULL, name TEXT NOT NULL DEFAULT 'Companion', species TEXT NOT NULL, stage TEXT NOT NULL, PRIMARY KEY(slot, pet_id));
            CREATE TABLE IF NOT EXISTS inventory(slot BLOB NOT NULL REFERENCES save_slots(slot) ON DELETE CASCADE, item_id TEXT NOT NULL, quantity INTEGER NOT NULL, PRIMARY KEY(slot, item_id));
            CREATE TABLE IF NOT EXISTS eggs(slot BLOB NOT NULL REFERENCES save_slots(slot) ON DELETE CASCADE, egg_id TEXT NOT NULL, species TEXT NOT NULL, laid_at TEXT NOT NULL, required_ticks TEXT NOT NULL, PRIMARY KEY(slot, egg_id));
            CREATE TABLE IF NOT EXISTS active_battles(slot BLOB NOT NULL REFERENCES save_slots(slot) ON DELETE CASCADE, battle_id TEXT NOT NULL, first_pet TEXT NOT NULL, second_pet TEXT NOT NULL, turn INTEGER NOT NULL, PRIMARY KEY(slot, battle_id));
            CREATE TABLE IF NOT EXISTS world_state(slot BLOB PRIMARY KEY REFERENCES save_slots(slot) ON DELETE CASCADE, tick TEXT NOT NULL, seed TEXT NOT NULL, balance INTEGER NOT NULL);").map_err(storage)?;
        let has_pet_name = connection
            .prepare("PRAGMA table_info(pets)")
            .map_err(storage)?
            .query_map([], |row| row.get::<_, String>(1))
            .map_err(storage)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage)?
            .iter()
            .any(|column| column == "name");
        if !has_pet_name {
            connection
                .execute(
                    "ALTER TABLE pets ADD COLUMN name TEXT NOT NULL DEFAULT 'Companion'",
                    [],
                )
                .map_err(storage)?;
        }
        Ok(())
    }

    fn acquire_lock(&self) -> Result<File, SaveError> {
        let lock_path = self.path.with_extension("lock");
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(lock_path)
            .map_err(storage)?;
        FileExt::try_lock(&file).map_err(|_| SaveError::Locked)?;
        Ok(file)
    }

    fn load_at(&self, path: &Path, slot: SaveSlotId) -> Result<GameState, SaveError> {
        let connection = Connection::open(path).map_err(storage)?;
        let bytes: Vec<u8> = connection
            .query_row(
                "SELECT envelope FROM game_snapshots WHERE slot=?1",
                params![slot.as_uuid().as_bytes().as_slice()],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?
            .ok_or(SaveError::NotFound)?;
        let envelope: SaveEnvelope =
            rmp_serde::from_slice(&bytes).map_err(|e| SaveError::Corrupt(e.to_string()))?;
        envelope.decode()
    }

    fn rotate_backups(&self) -> Result<(), SaveError> {
        if !self.path.exists() {
            return Ok(());
        }
        for index in (1..3).rev() {
            let from = backup_path(&self.path, index);
            let to = backup_path(&self.path, index + 1);
            if from.exists() {
                fs::copy(from, to).map_err(storage)?;
            }
        }
        fs::copy(&self.path, backup_path(&self.path, 1)).map_err(storage)?;
        let daily = PathBuf::from(format!("{}.daily", self.path.display()));
        let today = Utc::now().date_naive();
        let needs_daily = match fs::metadata(&daily).and_then(|metadata| metadata.modified()) {
            Ok(modified) => chrono::DateTime::<Utc>::from(modified).date_naive() != today,
            Err(_) => true,
        };
        if needs_daily {
            fs::copy(&self.path, daily).map_err(storage)?;
        }
        Ok(())
    }
}

impl SaveRepository for SqliteSaveRepository {
    fn list_slots(&self) -> Result<Vec<SlotSummary>, SaveError> {
        let connection = Connection::open(&self.path).map_err(storage)?;
        let mut statement = connection.prepare("SELECT slot, schema_version, updated_at FROM save_slots ORDER BY updated_at DESC, slot").map_err(storage)?;
        let rows = statement
            .query_map([], |row| {
                let bytes: Vec<u8> = row.get(0)?;
                let uuid = uuid::Uuid::from_slice(&bytes).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        bytes.len(),
                        rusqlite::types::Type::Blob,
                        Box::new(e),
                    )
                })?;
                Ok(SlotSummary {
                    slot: SaveSlotId::from_uuid(uuid),
                    schema_version: row.get(1)?,
                    updated_at: UnixSeconds(row.get(2)?),
                })
            })
            .map_err(storage)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(storage)
    }

    fn save(&self, slot: SaveSlotId, state: &GameState) -> Result<SaveMetadata, SaveError> {
        let lock = self.acquire_lock()?;
        let envelope = SaveEnvelope::encode(state)?;
        let encoded = rmp_serde::to_vec_named(&envelope).map_err(storage)?;
        let now = UnixSeconds(Utc::now().timestamp());
        let mut connection = Connection::open(&self.path).map_err(storage)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(storage)?;
        let slot_bytes = slot.as_uuid().as_bytes().to_vec();
        transaction.execute("INSERT INTO save_slots(slot,schema_version,updated_at) VALUES(?1,?2,?3) ON CONFLICT(slot) DO UPDATE SET schema_version=excluded.schema_version,updated_at=excluded.updated_at", params![&slot_bytes, GameState::SCHEMA_VERSION, now.0]).map_err(storage)?;
        transaction.execute("INSERT INTO game_snapshots(slot,envelope) VALUES(?1,?2) ON CONFLICT(slot) DO UPDATE SET envelope=excluded.envelope", params![&slot_bytes, encoded]).map_err(storage)?;
        transaction
            .execute("DELETE FROM pets WHERE slot=?1", params![&slot_bytes])
            .map_err(storage)?;
        transaction
            .execute("DELETE FROM inventory WHERE slot=?1", params![&slot_bytes])
            .map_err(storage)?;
        transaction
            .execute("DELETE FROM eggs WHERE slot=?1", params![&slot_bytes])
            .map_err(storage)?;
        transaction
            .execute(
                "DELETE FROM active_battles WHERE slot=?1",
                params![&slot_bytes],
            )
            .map_err(storage)?;
        {
            let mut insert = transaction
                .prepare("INSERT INTO pets(slot,pet_id,name,species,stage) VALUES(?1,?2,?3,?4,?5)")
                .map_err(storage)?;
            for pet in state.simulation.pets.values() {
                insert
                    .execute(params![
                        &slot_bytes,
                        pet.id.to_string(),
                        pet.name.as_str(),
                        pet.species.as_str(),
                        pet.stage.name()
                    ])
                    .map_err(storage)?;
            }
        }
        {
            let mut insert = transaction
                .prepare("INSERT INTO active_battles(slot,battle_id,first_pet,second_pet,turn) VALUES(?1,?2,?3,?4,?5)")
                .map_err(storage)?;
            for battle in state.active_battles.values() {
                insert
                    .execute(params![
                        &slot_bytes,
                        battle.id.to_string(),
                        battle.combatants[0].pet.to_string(),
                        battle.combatants[1].pet.to_string(),
                        battle.turn
                    ])
                    .map_err(storage)?;
            }
        }
        {
            let mut insert = transaction
                .prepare("INSERT INTO eggs(slot,egg_id,species,laid_at,required_ticks) VALUES(?1,?2,?3,?4,?5)")
                .map_err(storage)?;
            for egg in state.eggs.values() {
                insert
                    .execute(params![
                        &slot_bytes,
                        egg.id.to_string(),
                        egg.species.as_str(),
                        egg.incubation.laid_at.0.to_string(),
                        egg.incubation.required.as_ticks().to_string()
                    ])
                    .map_err(storage)?;
            }
        }
        {
            let mut insert = transaction
                .prepare("INSERT INTO inventory(slot,item_id,quantity) VALUES(?1,?2,?3)")
                .map_err(storage)?;
            for (item, quantity) in state.inventory.iter() {
                insert
                    .execute(params![&slot_bytes, item.as_str(), quantity])
                    .map_err(storage)?;
            }
        }
        transaction.execute("INSERT INTO world_state(slot,tick,seed,balance) VALUES(?1,?2,?3,?4) ON CONFLICT(slot) DO UPDATE SET tick=excluded.tick,seed=excluded.seed,balance=excluded.balance", params![&slot_bytes, state.simulation.tick.0.to_string(), state.simulation.seed.0.to_string(), state.wallet.balance().0]).map_err(storage)?;
        transaction.commit().map_err(storage)?;
        drop(connection);
        self.rotate_backups()?;
        FileExt::unlock(&lock).map_err(storage)?;
        Ok(SaveMetadata {
            schema_version: GameState::SCHEMA_VERSION,
            last_saved_at: now,
            clamped_offline_ticks: state.simulation.clamped_offline_ticks,
        })
    }

    fn load(&self, slot: SaveSlotId) -> Result<GameState, SaveError> {
        match self.load_at(&self.path, slot) {
            Err(SaveError::Corrupt(primary)) => {
                let lock = self.acquire_lock()?;
                if let Ok(current) = self.load_at(&self.path, slot) {
                    FileExt::unlock(&lock).map_err(storage)?;
                    return Ok(current);
                }
                let backup = backup_path(&self.path, 1);
                let recovered = self.load_at(&backup, slot).map_err(|backup_error| {
                    SaveError::Corrupt(format!("primary: {primary}; backup: {backup_error}"))
                })?;
                fs::copy(&backup, &self.path).map_err(storage)?;
                FileExt::unlock(&lock).map_err(storage)?;
                Ok(recovered)
            }
            result => result,
        }
    }

    fn delete(&self, slot: SaveSlotId) -> Result<(), SaveError> {
        let lock = self.acquire_lock()?;
        let connection = Connection::open(&self.path).map_err(storage)?;
        let changed = connection
            .execute(
                "DELETE FROM save_slots WHERE slot=?1",
                params![slot.as_uuid().as_bytes().as_slice()],
            )
            .map_err(storage)?;
        FileExt::unlock(&lock).map_err(storage)?;
        if changed == 0 {
            Err(SaveError::NotFound)
        } else {
            Ok(())
        }
    }
}

fn backup_path(path: &Path, index: usize) -> PathBuf {
    PathBuf::from(format!("{}.bak{index}", path.display()))
}
fn storage(error: impl std::fmt::Display) -> SaveError {
    SaveError::Storage(error.to_string())
}
