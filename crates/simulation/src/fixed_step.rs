//! The single tick path used by live play and offline catch-up.

use content::ContentRegistry;
use domain::events::DomainEvent;
use domain::time::GameDuration;

use crate::SimConfig;
use crate::scheduler::TickContext;
use crate::simulation_engine::SimulationState;

pub fn run_one(
    state: &mut SimulationState,
    content: &ContentRegistry,
    config: &SimConfig,
) -> Vec<DomainEvent> {
    state.tick = state.tick.saturating_add(GameDuration::from_ticks(1));
    let tick = state.tick;
    let seed = state.seed;
    let mut events = Vec::new();
    crate::scheduler::run(&mut TickContext {
        tick,
        state,
        content,
        seed,
        events: &mut events,
        config,
    });
    events
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedStepAccumulator {
    tick_millis: u64,
    pending_millis: u128,
}

impl FixedStepAccumulator {
    #[must_use]
    pub const fn new(tick_millis: u64) -> Self {
        Self {
            tick_millis,
            pending_millis: 0,
        }
    }
    pub fn push_millis(&mut self, elapsed_millis: u64) -> u64 {
        self.pending_millis = self
            .pending_millis
            .saturating_add(u128::from(elapsed_millis));
        if self.tick_millis == 0 {
            return 0;
        }
        let ticks = self.pending_millis / u128::from(self.tick_millis);
        self.pending_millis %= u128::from(self.tick_millis);
        u64::try_from(ticks).unwrap_or(u64::MAX)
    }
    #[must_use]
    pub fn pending_millis(&self) -> u128 {
        self.pending_millis
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn drains_only_whole_ticks() {
        let mut value = FixedStepAccumulator::new(100);
        assert_eq!(value.push_millis(99), 0);
        assert_eq!(value.push_millis(1), 1);
        assert_eq!(value.pending_millis(), 0);
    }
}
