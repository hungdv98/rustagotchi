//! Stable system order. Changing this array changes replay output.

use content::ContentRegistry;
use domain::events::DomainEvent;
use domain::rng::WorldSeed;
use domain::time::Tick;

use crate::SimConfig;
use crate::simulation_engine::SimulationState;
use crate::systems;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SystemId {
    NeedsDecay,
    EmotionalUpdate,
    HealthUpdate,
    IllnessUpdate,
    Aging,
    Growth,
    Evolution,
    Cooldowns,
    Habits,
    MemoryDecay,
    Behavior,
}

pub const SYSTEM_ORDER: [SystemId; 11] = [
    SystemId::NeedsDecay,
    SystemId::EmotionalUpdate,
    SystemId::HealthUpdate,
    SystemId::IllnessUpdate,
    SystemId::Aging,
    SystemId::Growth,
    SystemId::Evolution,
    SystemId::Cooldowns,
    SystemId::Habits,
    SystemId::MemoryDecay,
    SystemId::Behavior,
];

#[derive(Debug)]
pub struct TickContext<'a> {
    pub tick: Tick,
    pub state: &'a mut SimulationState,
    pub content: &'a ContentRegistry,
    pub seed: WorldSeed,
    pub events: &'a mut Vec<DomainEvent>,
    pub config: &'a SimConfig,
}

pub trait SimSystem {
    const ID: SystemId;
    fn run(ctx: &mut TickContext<'_>);
}

pub(crate) fn run(ctx: &mut TickContext<'_>) {
    for id in SYSTEM_ORDER {
        match id {
            SystemId::NeedsDecay => systems::NeedsDecay::run(ctx),
            SystemId::EmotionalUpdate => systems::EmotionalUpdate::run(ctx),
            SystemId::HealthUpdate => systems::HealthUpdate::run(ctx),
            SystemId::IllnessUpdate => systems::IllnessUpdate::run(ctx),
            SystemId::Aging => systems::Aging::run(ctx),
            SystemId::Growth => systems::Growth::run(ctx),
            SystemId::Evolution => systems::Evolution::run(ctx),
            SystemId::Cooldowns => systems::Cooldowns::run(ctx),
            SystemId::Habits => systems::Habits::run(ctx),
            SystemId::MemoryDecay => systems::MemoryDecay::run(ctx),
            SystemId::Behavior => systems::Behavior::run(ctx),
        }
    }
}
