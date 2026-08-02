use std::time::Duration;

use bevy::prelude::*;
use domain::combat::CombatEvent;
use domain::events::{BreedingEvent, DomainEvent, PetEvent};

use crate::DomainEventMessage;
use crate::screens::ScreenRoot;

const BASE_BACKGROUND: Color = Color::srgb(0.055, 0.07, 0.10);

#[derive(Clone, Copy, Debug, Eq, Message, PartialEq)]
pub enum FeedbackCue {
    Success,
    Warning,
    Impact,
}

#[derive(Clone, Copy, Debug, Resource)]
pub struct PresentationPreferences {
    pub audio_enabled: bool,
    pub reduced_motion: bool,
}

impl Default for PresentationPreferences {
    fn default() -> Self {
        Self {
            audio_enabled: true,
            reduced_motion: false,
        }
    }
}

#[derive(Debug, Resource)]
struct FeedbackPulse {
    timer: Timer,
    color: Color,
}

impl Default for FeedbackPulse {
    fn default() -> Self {
        Self {
            timer: Timer::new(Duration::ZERO, TimerMode::Once),
            color: BASE_BACKGROUND,
        }
    }
}

pub(crate) fn register(app: &mut App) {
    app.init_resource::<PresentationPreferences>()
        .init_resource::<FeedbackPulse>()
        .add_message::<FeedbackCue>()
        .add_systems(
            Update,
            (
                emit_feedback_cues,
                begin_feedback_pulse,
                animate_feedback_pulse,
            )
                .chain(),
        );
}

fn emit_feedback_cues(
    mut events: MessageReader<'_, '_, DomainEventMessage>,
    mut cues: MessageWriter<'_, FeedbackCue>,
) {
    for event in events.read() {
        if let Some(cue) = feedback_for(&event.0) {
            cues.write(cue);
        }
    }
}

fn begin_feedback_pulse(
    preferences: Res<'_, PresentationPreferences>,
    mut cues: MessageReader<'_, '_, FeedbackCue>,
    mut pulse: ResMut<'_, FeedbackPulse>,
) {
    for cue in cues.read() {
        if preferences.reduced_motion {
            continue;
        }
        pulse.timer = Timer::from_seconds(0.24, TimerMode::Once);
        pulse.color = match cue {
            FeedbackCue::Success => Color::srgb(0.08, 0.22, 0.16),
            FeedbackCue::Warning => Color::srgb(0.28, 0.09, 0.08),
            FeedbackCue::Impact => Color::srgb(0.24, 0.17, 0.06),
        };
    }
}

fn animate_feedback_pulse(
    time: Res<'_, Time>,
    preferences: Res<'_, PresentationPreferences>,
    mut pulse: ResMut<'_, FeedbackPulse>,
    mut roots: Query<'_, '_, &mut BackgroundColor, With<ScreenRoot>>,
) {
    let active = !preferences.reduced_motion && !pulse.timer.is_finished();
    if active {
        pulse.timer.tick(time.delta());
    }
    let color = if active && !pulse.timer.is_finished() {
        pulse.color
    } else {
        BASE_BACKGROUND
    };
    for mut background in &mut roots {
        background.0 = color;
    }
}

fn feedback_for(event: &DomainEvent) -> Option<FeedbackCue> {
    match event {
        DomainEvent::Pet(PetEvent::Hatched { .. } | PetEvent::NeedRecovered { .. })
        | DomainEvent::Breeding(
            BreedingEvent::EggCreated { .. } | BreedingEvent::EggHatched { .. },
        )
        | DomainEvent::Combat(CombatEvent::BattleWon { .. }) => Some(FeedbackCue::Success),
        DomainEvent::Pet(PetEvent::NeedBecameCritical { .. } | PetEvent::Died { .. }) => {
            Some(FeedbackCue::Warning)
        }
        DomainEvent::Combat(CombatEvent::DamageDealt { .. } | CombatEvent::Defeated { .. }) => {
            Some(FeedbackCue::Impact)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use domain::identifiers::{EggId, PetId};
    use domain::pet::NeedKind;
    use domain::rng::WorldSeed;
    use domain::time::Tick;

    use super::*;

    fn pet(ordinal: u64) -> PetId {
        PetId::derive(WorldSeed(8), Tick::ZERO, ordinal)
    }

    #[test]
    fn meaningful_events_map_to_distinct_feedback() {
        let success = DomainEvent::Breeding(BreedingEvent::EggCreated {
            egg: EggId::derive(WorldSeed(8), Tick::ZERO, 3),
            parents: [pet(1), pet(2)],
        });
        let warning = DomainEvent::Pet(PetEvent::NeedBecameCritical {
            pet: pet(1),
            need: NeedKind::Hunger,
            at: Tick(4),
        });
        let routine = DomainEvent::Pet(PetEvent::NeedChanged {
            pet: pet(1),
            need: NeedKind::Fun,
            from: domain::pet::NeedValue::FULL,
            to: domain::pet::NeedValue::new(domain::units::Milli(90_000)),
        });

        assert_eq!(feedback_for(&success), Some(FeedbackCue::Success));
        assert_eq!(feedback_for(&warning), Some(FeedbackCue::Warning));
        assert_eq!(feedback_for(&routine), None);
    }

    #[test]
    fn accessibility_preferences_have_safe_defaults() {
        let preferences = PresentationPreferences::default();
        assert!(preferences.audio_enabled);
        assert!(!preferences.reduced_motion);
    }
}
