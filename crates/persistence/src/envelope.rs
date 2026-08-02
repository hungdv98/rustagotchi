use application::{GameState, SaveError};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const CURRENT_FORMAT_VERSION: u16 = 1;
const MAGIC: [u8; 8] = *b"RGOTCHI\0";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveEnvelope {
    pub magic: [u8; 8],
    pub format_version: u16,
    pub schema_version: u32,
    pub created_by: [u16; 3],
    pub checksum: [u8; 32],
    pub payload: Vec<u8>,
}

impl SaveEnvelope {
    pub fn encode(state: &GameState) -> Result<Self, SaveError> {
        state.validate().map_err(SaveError::InvalidState)?;
        let message =
            rmp_serde::to_vec_named(state).map_err(|e| SaveError::Storage(e.to_string()))?;
        let payload = zstd::stream::encode_all(message.as_slice(), 3)
            .map_err(|e| SaveError::Storage(e.to_string()))?;
        let checksum = Sha256::digest(&payload).into();
        Ok(Self {
            magic: MAGIC,
            format_version: CURRENT_FORMAT_VERSION,
            schema_version: GameState::SCHEMA_VERSION,
            created_by: [0, 1, 0],
            checksum,
            payload,
        })
    }

    pub fn decode(&self) -> Result<GameState, SaveError> {
        if self.magic != MAGIC {
            return Err(SaveError::Corrupt("invalid envelope magic".into()));
        }
        if self.format_version != CURRENT_FORMAT_VERSION {
            return Err(SaveError::Corrupt(format!(
                "unsupported envelope format {}",
                self.format_version
            )));
        }
        if self.schema_version == 0 || self.schema_version > GameState::SCHEMA_VERSION {
            return Err(SaveError::UnsupportedSchema {
                found: self.schema_version,
                current: GameState::SCHEMA_VERSION,
            });
        }
        let actual: [u8; 32] = Sha256::digest(&self.payload).into();
        if actual != self.checksum {
            return Err(SaveError::Corrupt("snapshot checksum mismatch".into()));
        }
        let bytes = zstd::stream::decode_all(self.payload.as_slice())
            .map_err(|e| SaveError::Corrupt(e.to_string()))?;
        let state: GameState =
            rmp_serde::from_slice(&bytes).map_err(|e| SaveError::Corrupt(e.to_string()))?;
        let state = state
            .migrate_from(self.schema_version)
            .map_err(SaveError::InvalidState)?;
        state.validate().map_err(SaveError::InvalidState)?;
        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::rng::WorldSeed;
    use domain::time::UnixSeconds;
    #[test]
    fn round_trip_is_exact() {
        let state = GameState::new(WorldSeed(1), UnixSeconds(0));
        assert_eq!(
            SaveEnvelope::encode(&state).unwrap().decode().unwrap(),
            state
        );
    }
    #[test]
    fn bit_flip_is_detected() {
        let state = GameState::new(WorldSeed(1), UnixSeconds(0));
        let mut value = SaveEnvelope::encode(&state).unwrap();
        value.payload[0] ^= 1;
        assert!(matches!(value.decode(), Err(SaveError::Corrupt(_))));
    }

    #[test]
    fn schema_one_payload_migrates_to_current_state() {
        let state = GameState::new(WorldSeed(1), UnixSeconds(0));
        let mut legacy = serde_json::to_value(&state).unwrap();
        let object = legacy.as_object_mut().unwrap();
        object.remove("eggs");
        object.remove("breeding_cooldowns");
        object.remove("active_battles");
        object["metadata"]["schema_version"] = serde_json::json!(1);
        let message = rmp_serde::to_vec_named(&legacy).unwrap();
        let payload = zstd::stream::encode_all(message.as_slice(), 3).unwrap();
        let envelope = SaveEnvelope {
            magic: MAGIC,
            format_version: CURRENT_FORMAT_VERSION,
            schema_version: 1,
            created_by: [0, 1, 0],
            checksum: Sha256::digest(&payload).into(),
            payload,
        };

        let migrated = envelope.decode().unwrap();
        assert_eq!(migrated.metadata.schema_version, GameState::SCHEMA_VERSION);
        assert!(migrated.eggs.is_empty());
        assert!(migrated.breeding_cooldowns.is_empty());
    }
}
