//! Offline catch-up, intentionally implemented by repeated live ticks.

use content::ContentRegistry;
use domain::events::DomainEvent;
use domain::time::{GameDuration, MAX_OFFLINE_CATCHUP_TICKS};

use crate::simulation_engine::SimulationState;
use crate::{SimConfig, fixed_step};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatchUpReport {
    pub requested_ticks: u64,
    pub simulated_ticks: u64,
    pub clamped_ticks: u64,
    pub events: Vec<DomainEvent>,
    pub dropped_events: u64,
}

pub fn catch_up(
    state: &mut SimulationState,
    elapsed: GameDuration,
    content: &ContentRegistry,
    config: &SimConfig,
) -> CatchUpReport {
    catch_up_with_progress(state, elapsed, content, config, |_, _| {})
}

pub fn catch_up_with_progress(
    state: &mut SimulationState,
    elapsed: GameDuration,
    content: &ContentRegistry,
    config: &SimConfig,
    mut progress: impl FnMut(u64, u64),
) -> CatchUpReport {
    let requested_ticks = elapsed.as_ticks();
    let simulated_ticks = requested_ticks.min(MAX_OFFLINE_CATCHUP_TICKS);
    let clamped_ticks = requested_ticks - simulated_ticks;
    state.clamped_offline_ticks = state.clamped_offline_ticks.saturating_add(clamped_ticks);
    const MAX_REPORTED_EVENTS: usize = 4_096;
    let mut events = Vec::new();
    let mut dropped_events = 0u64;
    for completed in 0..simulated_ticks {
        for event in fixed_step::run_one(state, content, config) {
            if events.len() < MAX_REPORTED_EVENTS {
                events.push(event);
            } else {
                dropped_events = dropped_events.saturating_add(1);
            }
        }
        if completed % 10_000 == 0 || completed + 1 == simulated_ticks {
            progress(completed + 1, simulated_ticks);
        }
    }
    CatchUpReport {
        requested_ticks,
        simulated_ticks,
        clamped_ticks,
        events,
        dropped_events,
    }
}
