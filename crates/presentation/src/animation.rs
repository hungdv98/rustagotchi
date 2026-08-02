use std::time::Duration;

use bevy::math::ops;
use bevy::prelude::*;

use crate::{FeedbackCue, GameViewModel, PresentationPreferences};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum ActivityAnimation {
    Idle,
    Eating,
    Sleeping,
    Playing,
    Exploring,
    Socialising,
    Bathing,
    Battling,
    Recovering,
}

impl ActivityAnimation {
    fn from_name(name: &str) -> Self {
        match name {
            "eating" => Self::Eating,
            "sleeping" => Self::Sleeping,
            "playing" => Self::Playing,
            "exploring" => Self::Exploring,
            "socialising" => Self::Socialising,
            "bathing" => Self::Bathing,
            "battling" => Self::Battling,
            "recovering" => Self::Recovering,
            _ => Self::Idle,
        }
    }
}

#[derive(Component, Debug)]
pub(crate) struct PetAnimator {
    pet_index: usize,
    activity: ActivityAnimation,
}

impl PetAnimator {
    pub(crate) fn new(pet_index: usize) -> Self {
        Self {
            pet_index,
            activity: ActivityAnimation::Idle,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Reaction {
    Celebrate,
    Alarm,
    Impact,
}

#[derive(Debug, Default, Resource)]
struct AnimationDirector {
    reaction: Option<Reaction>,
    timer: Timer,
}

pub(crate) fn register(app: &mut App) {
    app.init_resource::<AnimationDirector>().add_systems(
        Update,
        (
            update_activity_animation,
            begin_event_reaction,
            animate_pet_portrait,
        )
            .chain(),
    );
}

fn update_activity_animation(
    view: Res<'_, GameViewModel>,
    mut portraits: Query<'_, '_, &mut PetAnimator>,
) {
    for mut animator in &mut portraits {
        let pet = view
            .summary
            .as_ref()
            .and_then(|summary| summary.pets.get(animator.pet_index));
        animator.activity = pet.map_or(ActivityAnimation::Idle, |pet| {
            ActivityAnimation::from_name(&pet.activity)
        });
    }
}

fn begin_event_reaction(
    preferences: Res<'_, PresentationPreferences>,
    mut cues: MessageReader<'_, '_, FeedbackCue>,
    mut director: ResMut<'_, AnimationDirector>,
) {
    for cue in cues.read() {
        if preferences.reduced_motion {
            continue;
        }
        director.reaction = Some(match cue {
            FeedbackCue::Success => Reaction::Celebrate,
            FeedbackCue::Warning => Reaction::Alarm,
            FeedbackCue::Impact => Reaction::Impact,
        });
        director.timer = Timer::new(Duration::from_millis(420), TimerMode::Once);
    }
}

fn animate_pet_portrait(
    time: Res<'_, Time>,
    preferences: Res<'_, PresentationPreferences>,
    mut director: ResMut<'_, AnimationDirector>,
    mut portraits: Query<'_, '_, (&PetAnimator, &mut UiTransform)>,
) {
    if preferences.reduced_motion {
        director.reaction = None;
        for (_, mut transform) in &mut portraits {
            *transform = UiTransform::IDENTITY;
        }
        return;
    }

    if director.reaction.is_some() {
        director.timer.tick(time.delta());
        if director.timer.is_finished() {
            director.reaction = None;
        }
    }
    let elapsed = time.elapsed_secs();
    for (animator, mut transform) in &mut portraits {
        let (x, y, rotation, scale) = director.reaction.map_or_else(
            || activity_transform(animator.activity, elapsed),
            |reaction| reaction_transform(reaction, elapsed),
        );
        transform.translation = Val2::px(x, y);
        transform.rotation = Rot2::radians(rotation);
        transform.scale = Vec2::splat(scale);
    }
}

fn activity_transform(activity: ActivityAnimation, elapsed: f32) -> (f32, f32, f32, f32) {
    let wave = ops::sin(elapsed * 2.4);
    match activity {
        ActivityAnimation::Idle => (0.0, wave * 2.5, 0.0, 1.0),
        ActivityAnimation::Eating => (0.0, 0.0, wave * 0.025, 1.0 + wave.abs() * 0.025),
        ActivityAnimation::Sleeping => (0.0, 2.0, -0.04, 0.98 + wave * 0.008),
        ActivityAnimation::Playing | ActivityAnimation::Socialising => {
            (wave * 3.0, wave.abs() * -4.0, wave * 0.035, 1.0)
        }
        ActivityAnimation::Exploring => (wave * 4.0, 0.0, 0.0, 1.0),
        ActivityAnimation::Bathing => (0.0, wave * 1.5, wave * 0.02, 1.0),
        ActivityAnimation::Battling => (wave.abs() * 5.0, 0.0, wave * 0.03, 1.02),
        ActivityAnimation::Recovering => (0.0, 3.0, -0.025, 0.96),
    }
}

fn reaction_transform(reaction: Reaction, elapsed: f32) -> (f32, f32, f32, f32) {
    let fast_wave = ops::sin(elapsed * 28.0);
    match reaction {
        Reaction::Celebrate => (0.0, -fast_wave.abs() * 8.0, 0.0, 1.06),
        Reaction::Alarm => (fast_wave * 5.0, 0.0, fast_wave * 0.045, 1.0),
        Reaction::Impact => (fast_wave.abs() * 8.0, 0.0, -0.04, 0.97),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_authored_activity_has_a_distinct_supported_state() {
        let names = [
            "idle",
            "eating",
            "sleeping",
            "playing",
            "exploring",
            "socialising",
            "bathing",
            "battling",
            "recovering",
        ];
        let states: std::collections::BTreeSet<_> = names
            .into_iter()
            .map(ActivityAnimation::from_name)
            .collect();
        assert_eq!(states.len(), names.len());
    }

    #[test]
    fn reduced_motion_can_reset_to_the_identity_transform() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<PresentationPreferences>()
            .init_resource::<GameViewModel>()
            .init_resource::<AnimationDirector>()
            .add_systems(Update, animate_pet_portrait);
        app.world_mut()
            .resource_mut::<PresentationPreferences>()
            .reduced_motion = true;
        let entity = app
            .world_mut()
            .spawn((
                PetAnimator::new(0),
                UiTransform::from_scale(Vec2::splat(1.5)),
            ))
            .id();

        app.update();

        assert_eq!(
            *app.world().get::<UiTransform>(entity).unwrap(),
            UiTransform::IDENTITY
        );
    }
}
