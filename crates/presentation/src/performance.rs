use std::time::Duration;

use bevy::prelude::*;

use crate::{AppState, GameViewModel};

const TARGET_PET_COUNT: usize = 8;
const TARGET_FRAME_MICROS: u64 = 16_667;
const WARMUP_FRAMES: usize = 60;
const SAMPLE_FRAMES: usize = 240;

pub const EIGHT_PET_FRAME_BUDGET: Duration = Duration::from_nanos(16_666_667);

#[derive(Clone, Copy, Debug, Default, Resource)]
pub struct FramePerformanceSnapshot {
    pub sample_count: usize,
    pub average_frame_micros: u64,
    pub p95_frame_micros: u64,
    pub budget_micros: u64,
    pub within_budget: bool,
}

#[derive(Debug, Default, Resource)]
struct FrameSamples {
    warmup: usize,
    micros: Vec<u64>,
}

pub(crate) fn register(app: &mut App) {
    app.init_resource::<FramePerformanceSnapshot>()
        .init_resource::<FrameSamples>()
        .add_systems(Update, sample_eight_pet_frame_time);
}

fn sample_eight_pet_frame_time(
    time: Res<'_, Time>,
    state: Res<'_, State<AppState>>,
    view: Res<'_, GameViewModel>,
    mut samples: ResMut<'_, FrameSamples>,
    mut snapshot: ResMut<'_, FramePerformanceSnapshot>,
) {
    let target_is_visible = *state.get() == AppState::Home
        && view
            .summary
            .as_ref()
            .is_some_and(|summary| summary.pets.len() >= TARGET_PET_COUNT);
    if !target_is_visible {
        samples.warmup = 0;
        samples.micros.clear();
        return;
    }
    if samples.warmup < WARMUP_FRAMES {
        samples.warmup += 1;
        return;
    }
    let micros = u64::try_from(time.delta().as_micros()).unwrap_or(u64::MAX);
    if micros == 0 {
        return;
    }
    samples.micros.push(micros);
    if samples.micros.len() < SAMPLE_FRAMES {
        return;
    }

    *snapshot = summarize(&samples.micros, TARGET_FRAME_MICROS);
    if snapshot.within_budget {
        tracing::debug!(
            pets = TARGET_PET_COUNT,
            average_frame_micros = snapshot.average_frame_micros,
            p95_frame_micros = snapshot.p95_frame_micros,
            "presentation frame budget met"
        );
    } else {
        tracing::warn!(
            pets = TARGET_PET_COUNT,
            average_frame_micros = snapshot.average_frame_micros,
            p95_frame_micros = snapshot.p95_frame_micros,
            budget_micros = snapshot.budget_micros,
            "presentation frame budget exceeded"
        );
    }
    samples.micros.clear();
}

fn summarize(samples: &[u64], budget_micros: u64) -> FramePerformanceSnapshot {
    if samples.is_empty() {
        return FramePerformanceSnapshot {
            budget_micros,
            ..default()
        };
    }
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let p95_index = (sorted.len() * 95).div_ceil(100).saturating_sub(1);
    let total = samples
        .iter()
        .fold(0_u128, |sum, sample| sum + u128::from(*sample));
    let average = total / samples.len() as u128;
    let average_frame_micros = u64::try_from(average).unwrap_or(u64::MAX);
    let p95_frame_micros = sorted[p95_index];
    FramePerformanceSnapshot {
        sample_count: samples.len(),
        average_frame_micros,
        p95_frame_micros,
        budget_micros,
        within_budget: p95_frame_micros <= budget_micros,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn p95_budget_rejects_sustained_slow_frames_without_chasing_one_outlier() {
        let mut samples = vec![16_000; 95];
        samples.extend([30_000; 5]);
        let passing = summarize(&samples, TARGET_FRAME_MICROS);
        assert!(passing.within_budget);
        assert_eq!(passing.p95_frame_micros, 16_000);

        samples[90] = 30_000;
        let failing = summarize(&samples, TARGET_FRAME_MICROS);
        assert!(!failing.within_budget);
        assert_eq!(failing.p95_frame_micros, 30_000);
    }

    #[test]
    fn empty_samples_are_reported_without_division_by_zero() {
        let snapshot = summarize(&[], TARGET_FRAME_MICROS);
        assert_eq!(snapshot.sample_count, 0);
        assert!(!snapshot.within_budget);
    }
}
