use serde::{Deserialize, Serialize};

use super::{Element, StatusEffect, StatusKind};
use crate::identifiers::{BattleId, MoveId, PetId};
use crate::pet::Stats;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Combatant {
    pub pet: PetId,
    pub element: Element,
    pub stats: Stats,
    pub health: u32,
    pub energy: u16,
    pub statuses: Vec<StatusEffect>,
}

impl Combatant {
    #[must_use]
    pub fn from_stats(pet: PetId, element: Element, stats: Stats) -> Self {
        Self {
            pet,
            element,
            stats,
            health: u32::from(stats.vitality.get()).saturating_mul(10),
            energy: 100,
            statuses: Vec::new(),
        }
    }

    #[must_use]
    pub const fn is_defeated(&self) -> bool {
        self.health == 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattleState {
    pub id: BattleId,
    pub combatants: [Combatant; 2],
    pub turn: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattleAction {
    pub actor: PetId,
    pub move_id: MoveId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CombatEvent {
    BattleStarted {
        battle: BattleId,
        combatants: [PetId; 2],
    },
    MoveMissed {
        actor: PetId,
        move_id: MoveId,
    },
    DamageDealt {
        actor: PetId,
        target: PetId,
        move_id: MoveId,
        amount: u32,
    },
    Defeated {
        pet: PetId,
    },
    BattleWon {
        battle: BattleId,
        winner: PetId,
        loser: PetId,
        coins: i64,
        experience: u32,
    },
    StatusApplied {
        target: PetId,
        status: StatusKind,
        turns: u8,
    },
    StatusPreventedAction {
        pet: PetId,
        status: StatusKind,
    },
    StatusDamaged {
        pet: PetId,
        status: StatusKind,
        amount: u32,
    },
    StatusExpired {
        pet: PetId,
        status: StatusKind,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattleResult {
    pub winner: PetId,
    pub loser: PetId,
    pub turns: u32,
    pub final_state: BattleState,
    pub events: Vec<CombatEvent>,
}
