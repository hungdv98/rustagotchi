//! Rustagotchi desktop composition root.

mod bootstrap;

use std::time::Duration;

use anyhow::Result;
use application::Command;
use application::ports::{Clock, Notification, NotificationService};
use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use bevy::window::{Window, WindowPlugin, WindowResizeConstraints};
use platform::{DesktopNotifications, SystemClock};
use presentation::{
    AppCommand, DomainEventMessage, FeedbackCue, PresentationPreferences,
    RustagotchiPresentationPlugin,
};
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    init_tracing();
    let bootstrap = bootstrap::Bootstrap::load()?;
    let asset_path = bootstrap.assets.to_string_lossy().into_owned();
    let startup_events = StartupDomainEvents(bootstrap.startup_events);

    App::new()
        .insert_resource(bootstrap.engine)
        .insert_resource(bootstrap.slot)
        .insert_resource(bootstrap.process_lock)
        .insert_resource(startup_events)
        .insert_resource(WallClockPoll(Timer::from_seconds(
            1.0,
            TimerMode::Repeating,
        )))
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: asset_path,
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(primary_window()),
                    ..default()
                }),
        )
        .add_plugins(RustagotchiPresentationPlugin)
        .add_systems(Startup, emit_startup_events)
        .add_systems(
            Update,
            (poll_wall_clock, notify_domain_events, play_feedback_audio),
        )
        .run();
    Ok(())
}

fn primary_window() -> Window {
    Window {
        title: "Rustagotchi".into(),
        resolution: (1_100, 720).into(),
        resize_constraints: WindowResizeConstraints {
            min_width: 360.0,
            min_height: 540.0,
            ..default()
        },
        resizable: true,
        ..default()
    }
}

fn notify_domain_events(mut events: MessageReader<'_, '_, DomainEventMessage>) {
    for event in events.read() {
        if let Some(notification) = notification_for(&event.0) {
            DesktopNotifications.notify(notification);
        }
    }
}

fn play_feedback_audio(
    preferences: Res<'_, PresentationPreferences>,
    mut cues: MessageReader<'_, '_, FeedbackCue>,
    mut pitches: ResMut<'_, Assets<Pitch>>,
    mut commands: Commands<'_, '_>,
) {
    for cue in cues.read() {
        if !preferences.audio_enabled {
            continue;
        }
        let frequency = match cue {
            FeedbackCue::Success => 660.0,
            FeedbackCue::Warning => 220.0,
            FeedbackCue::Impact => 330.0,
        };
        commands.spawn((
            AudioPlayer(pitches.add(Pitch::new(frequency, Duration::from_millis(90)))),
            PlaybackSettings::DESPAWN,
        ));
    }
}

fn notification_for(event: &domain::events::DomainEvent) -> Option<Notification> {
    match event {
        domain::events::DomainEvent::Pet(domain::events::PetEvent::NeedBecameCritical {
            pet,
            need,
            ..
        }) => Some(Notification {
            title: "A pet needs attention".into(),
            body: format!("Pet {pet}'s {need} need is critical."),
        }),
        domain::events::DomainEvent::Pet(domain::events::PetEvent::Died { pet, .. }) => {
            Some(Notification {
                title: "A pet has died".into(),
                body: format!("Pet {pet} has died."),
            })
        }
        _ => None,
    }
}

#[derive(Debug, Resource)]
struct WallClockPoll(Timer);

#[derive(Debug, Resource)]
struct StartupDomainEvents(Vec<domain::events::DomainEvent>);

fn emit_startup_events(
    mut startup: ResMut<'_, StartupDomainEvents>,
    mut events: MessageWriter<'_, DomainEventMessage>,
) {
    for event in startup.0.drain(..) {
        events.write(DomainEventMessage(event));
    }
}

fn poll_wall_clock(
    time: Res<'_, Time>,
    mut poll: ResMut<'_, WallClockPoll>,
    mut commands: MessageWriter<'_, AppCommand>,
) {
    if poll.0.tick(time.delta()).just_finished() {
        commands.write(AppCommand(Command::AdvanceTo {
            wall_now: SystemClock.now_unix(),
        }));
    }
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

#[cfg(test)]
mod tests {
    use domain::events::{DomainEvent, PetEvent};
    use domain::identifiers::PetId;
    use domain::pet::NeedKind;
    use domain::rng::WorldSeed;
    use domain::time::Tick;

    use super::*;

    #[test]
    fn only_actionable_pet_events_become_native_notifications() {
        let pet = PetId::derive(WorldSeed(1), Tick::ZERO, 1);
        let critical = DomainEvent::Pet(PetEvent::NeedBecameCritical {
            pet,
            need: NeedKind::Hunger,
            at: Tick(4),
        });
        assert!(notification_for(&critical).unwrap().body.contains("hunger"));

        let routine = DomainEvent::Pet(PetEvent::NeedChanged {
            pet,
            need: NeedKind::Hunger,
            from: domain::pet::NeedValue::FULL,
            to: domain::pet::NeedValue::new(domain::units::Milli(90_000)),
        });
        assert!(notification_for(&routine).is_none());
    }

    #[test]
    fn desktop_window_cannot_shrink_below_the_supported_layout() {
        let window = primary_window();
        assert_eq!(window.resize_constraints.min_width, 360.0);
        assert_eq!(window.resize_constraints.min_height, 540.0);
        assert!(window.resizable);
    }
}
