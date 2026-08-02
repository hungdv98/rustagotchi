use application::dto::{BattleSetupDto, GameSummaryDto, SaveSlotDto};
use bevy::prelude::*;

#[derive(Clone, Debug, Default, Resource)]
pub struct GameViewModel {
    pub summary: Option<GameSummaryDto>,
    pub battle_setup: Option<BattleSetupDto>,
    pub last_battle_result: Option<String>,
    pub save_slots: Vec<SaveSlotDto>,
    pub last_notice: Option<String>,
    pub last_error: Option<String>,
}
