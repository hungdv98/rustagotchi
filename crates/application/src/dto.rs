//! Stable, presentation-facing views of game state.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameSummaryDto {
    pub tick: u64,
    pub balance: i64,
    pub clamped_offline_ticks: u64,
    pub adopted_pets: usize,
    pub max_adopted_pets: usize,
    pub maximum_adopted_pets: usize,
    pub next_adoption_slot_cost: Option<i64>,
    pub pets: Vec<PetDto>,
    pub eggs: Vec<EggDto>,
    pub active_battles: Vec<BattleDto>,
    pub inventory: Vec<InventoryItemDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattleDto {
    pub id: String,
    pub combatant_ids: [String; 2],
    pub health: [u32; 2],
    pub energy: [u16; 2],
    pub turn: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EggDto {
    pub id: String,
    pub species: String,
    pub parent_ids: [String; 2],
    pub remaining_ticks: u64,
    pub ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PetDto {
    pub id: String,
    pub name: String,
    pub species: String,
    pub life_stage: String,
    pub mood: String,
    pub activity: String,
    pub alive: bool,
    pub age_ticks: u64,
    pub level: u16,
    pub experience: u32,
    pub experience_for_next_level: u32,
    pub experience_progress_milli: i32,
    pub needs: Vec<NeedDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NeedDto {
    pub kind: String,
    /// Satisfaction in thousandths of a percent (`100_000` is full).
    pub value_milli: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventoryItemDto {
    pub item_id: String,
    pub quantity: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattleSetupDto {
    pub pets: Vec<BattlePetDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattlePetDto {
    pub id: String,
    pub name: String,
    pub species: String,
    pub moves: Vec<BattleMoveDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattleMoveDto {
    pub id: String,
    pub power: u16,
    pub accuracy_milli: i32,
    pub energy_cost: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveSlotDto {
    pub id: String,
    pub schema_version: u32,
    pub updated_at_unix: i64,
}
