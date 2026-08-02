use std::collections::BTreeMap;

use domain::breeding::Egg;
use domain::combat::BattleState;
use domain::economy::Wallet;
use domain::identifiers::{BattleId, EggId, PetId};
use domain::inventory::Inventory;
use domain::rng::WorldSeed;
use domain::time::{GameClock, UnixSeconds};
use serde::{Deserialize, Serialize};
use simulation::SimulationState;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveMetadata {
    pub schema_version: u32,
    pub last_saved_at: UnixSeconds,
    pub clamped_offline_ticks: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameState {
    pub simulation: SimulationState,
    pub inventory: Inventory,
    pub wallet: Wallet,
    /// Permanent adoption-capacity upgrades bought with coins.
    #[serde(default)]
    pub adoption_slot_upgrades: u8,
    #[serde(default)]
    pub eggs: BTreeMap<EggId, Egg>,
    #[serde(default)]
    pub breeding_cooldowns: BTreeMap<PetId, domain::time::Tick>,
    #[serde(default)]
    pub active_battles: BTreeMap<BattleId, BattleState>,
    pub clock: GameClock,
    pub next_entity_ordinal: u64,
    pub metadata: SaveMetadata,
}

impl GameState {
    pub const SCHEMA_VERSION: u32 = 5;
    #[must_use]
    pub fn new(seed: WorldSeed, wall_now: UnixSeconds) -> Self {
        Self {
            simulation: SimulationState::new(seed),
            inventory: Inventory::default(),
            wallet: Wallet::default(),
            adoption_slot_upgrades: 0,
            eggs: BTreeMap::new(),
            breeding_cooldowns: BTreeMap::new(),
            active_battles: BTreeMap::new(),
            clock: GameClock::starting_at(wall_now),
            next_entity_ordinal: 1,
            metadata: SaveMetadata {
                schema_version: Self::SCHEMA_VERSION,
                last_saved_at: wall_now,
                clamped_offline_ticks: 0,
            },
        }
    }

    pub fn migrate_from(mut self, source_schema: u32) -> Result<Self, String> {
        match source_schema {
            1..=4 => {
                self.metadata.schema_version = Self::SCHEMA_VERSION;
                Ok(self)
            }
            version if version == Self::SCHEMA_VERSION => Ok(self),
            version => Err(format!("unsupported source schema {version}")),
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.metadata.schema_version != Self::SCHEMA_VERSION {
            return Err(format!(
                "metadata schema {} does not match {}",
                self.metadata.schema_version,
                Self::SCHEMA_VERSION
            ));
        }
        if self.clock.current() != self.simulation.tick {
            return Err(format!(
                "clock {} disagrees with simulation {}",
                self.clock.current(),
                self.simulation.tick
            ));
        }
        if self.inventory.distinct_kinds() > self.inventory.capacity() {
            return Err("inventory exceeds its capacity".into());
        }
        if self.wallet.balance().0 < 0 {
            return Err("wallet balance is negative".into());
        }
        if self.adoption_slot_upgrades > crate::MAX_ADOPTION_SLOT_UPGRADES {
            return Err("adoption slot upgrades exceed the supported maximum".into());
        }
        for (id, egg) in &self.eggs {
            if *id != egg.id {
                return Err(format!("egg map key {id} disagrees with egg {}", egg.id));
            }
            if egg.parents[0] == egg.parents[1] {
                return Err(format!("egg {id} has the same parent twice"));
            }
        }
        for (id, battle) in &self.active_battles {
            if *id != battle.id {
                return Err(format!(
                    "battle map key {id} disagrees with battle {}",
                    battle.id
                ));
            }
            if battle.combatants[0].pet == battle.combatants[1].pet {
                return Err(format!("battle {id} has the same combatant twice"));
            }
        }
        Ok(())
    }
}
