use application::dto::{BattleSetupDto, GameSummaryDto, SaveSlotDto};
use bevy::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BattleResultView {
    pub winner_name: String,
    pub coins_earned: i64,
    pub balance: i64,
    pub experience_earned: u32,
    pub level: u16,
    pub experience: u32,
    pub experience_for_next_level: u32,
    pub levels_gained: u16,
}

#[derive(Clone, Debug, Default, Resource)]
pub struct GameViewModel {
    pub summary: Option<GameSummaryDto>,
    pub battle_setup: Option<BattleSetupDto>,
    pub last_battle_result: Option<BattleResultView>,
    pub save_slots: Vec<SaveSlotDto>,
    pub last_notice: Option<String>,
    pub last_error: Option<String>,
}
