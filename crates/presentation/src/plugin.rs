use application::{BattleSetupQuery, Command, GameSummaryQuery};
use bevy::input_focus::tab_navigation::TabNavigationPlugin;
use bevy::prelude::*;

use crate::animation;
use crate::events::{AppCommand, CommandFailed, DomainEventMessage, SwitchSaveSlot};
use crate::feedback;
use crate::performance;
use crate::resources::{ActiveSaveSlot, EngineResource, PetSelection};
use crate::screens;
use crate::{AppState, BattleResultView, GameViewModel};

#[derive(Debug, Default)]
pub struct RustagotchiPresentationPlugin;

impl Plugin for RustagotchiPresentationPlugin {
    fn build(&self, app: &mut App) {
        animation::register(app);
        feedback::register(app);
        performance::register(app);
        app.init_state::<AppState>()
            .add_plugins(TabNavigationPlugin)
            .init_resource::<GameViewModel>()
            .init_resource::<PetSelection>()
            .add_message::<AppCommand>()
            .add_message::<DomainEventMessage>()
            .add_message::<CommandFailed>()
            .add_message::<SwitchSaveSlot>()
            .add_message::<screens::MenuActivated>()
            .add_observer(screens::activate_menu_button)
            .add_systems(Startup, (initialize_view_model, setup_camera))
            .add_systems(
                Update,
                (
                    queue_save_on_exit,
                    dispatch_commands,
                    enter_after_pet_roster_change,
                    record_battle_result,
                    record_errors,
                    screens::navigate_buttons,
                    screens::refresh_breeding_screen,
                    screens::refresh_battle_screen,
                    screens::handle_preference_buttons,
                    screens::refresh_pet_status_text,
                    screens::refresh_need_meters,
                    screens::refresh_care_action,
                    screens::refresh_breeding_action,
                    screens::refresh_pet_cycle_action,
                    screens::style_buttons,
                    screens::refresh_inventory_text,
                    screens::refresh_save_slot_text,
                    screens::refresh_home_text,
                    screens::refresh_status_text,
                    screens::refresh_preference_text,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                (
                    screens::refresh_home_pets,
                    screens::refresh_pet_status_portrait,
                    screens::refresh_preference_buttons,
                ),
            )
            .add_systems(OnEnter(AppState::Loading), finish_loading)
            .add_systems(OnEnter(AppState::MainMenu), screens::spawn_main_menu)
            .add_systems(OnExit(AppState::MainMenu), screens::despawn_screen)
            .add_systems(OnEnter(AppState::SaveSlots), screens::spawn_save_slots)
            .add_systems(OnExit(AppState::SaveSlots), screens::despawn_screen)
            .add_systems(
                OnEnter(AppState::ResetConfirmation),
                screens::spawn_reset_confirmation,
            )
            .add_systems(OnExit(AppState::ResetConfirmation), screens::despawn_screen)
            .add_systems(OnEnter(AppState::PetCreation), screens::spawn_pet_creation)
            .add_systems(OnExit(AppState::PetCreation), screens::despawn_screen)
            .add_systems(OnEnter(AppState::Home), screens::spawn_home)
            .add_systems(OnExit(AppState::Home), screens::despawn_screen)
            .add_systems(OnEnter(AppState::PetStatus), screens::spawn_pet_status)
            .add_systems(OnExit(AppState::PetStatus), screens::despawn_screen)
            .add_systems(OnEnter(AppState::PetRelease), screens::spawn_pet_release)
            .add_systems(OnExit(AppState::PetRelease), screens::despawn_screen)
            .add_systems(OnEnter(AppState::Inventory), screens::spawn_inventory)
            .add_systems(OnExit(AppState::Inventory), screens::despawn_screen)
            .add_systems(OnEnter(AppState::Breeding), screens::spawn_breeding)
            .add_systems(OnExit(AppState::Breeding), screens::despawn_screen)
            .add_systems(OnEnter(AppState::Battle), screens::spawn_battle)
            .add_systems(OnExit(AppState::Battle), screens::despawn_screen);
    }
}

fn initialize_view_model(engine: Res<'_, EngineResource>, mut view: ResMut<'_, GameViewModel>) {
    if let Err(error) = refresh_view_model(&engine, &mut view) {
        view.last_error = Some(error.to_string());
    }
}

fn setup_camera(mut commands: Commands<'_, '_>) {
    commands.spawn(Camera2d);
}

fn finish_loading(mut next: ResMut<'_, NextState<AppState>>) {
    next.set(AppState::MainMenu);
}

fn queue_save_on_exit(
    mut exits: MessageReader<'_, '_, AppExit>,
    slot: Res<'_, ActiveSaveSlot>,
    mut commands: MessageWriter<'_, AppCommand>,
) {
    if exits.read().next().is_some() {
        commands.write(AppCommand(Command::Save { slot: slot.0 }));
    }
}

fn dispatch_commands(
    mut engine: ResMut<'_, EngineResource>,
    mut commands: MessageReader<'_, '_, AppCommand>,
    mut switches: MessageReader<'_, '_, SwitchSaveSlot>,
    mut events: MessageWriter<'_, DomainEventMessage>,
    mut failures: MessageWriter<'_, CommandFailed>,
    mut view: ResMut<'_, GameViewModel>,
    mut active_slot: ResMut<'_, ActiveSaveSlot>,
) {
    for command in commands.read() {
        match engine.0.dispatch(command.0.clone()) {
            Ok(produced) => {
                for event in produced {
                    events.write(DomainEventMessage(event));
                }
                match refresh_view_model(&engine, &mut view) {
                    Ok(()) => record_command_success(&command.0, &mut view),
                    Err(error) => {
                        failures.write(error.into());
                    }
                }
            }
            Err(error) => {
                failures.write(CommandFailed::for_command(&command.0, error));
            }
        }
    }
    for switch in switches.read() {
        match engine.0.dispatch(Command::Load { slot: switch.0 }) {
            Ok(_) => {
                active_slot.0 = switch.0;
                view.last_battle_result = None;
                match refresh_view_model(&engine, &mut view) {
                    Ok(()) => {
                        view.last_error = None;
                        view.last_notice = Some("Save slot loaded.".into());
                    }
                    Err(error) => {
                        failures.write(error.into());
                    }
                }
            }
            Err(error) => {
                failures.write(error.into());
            }
        }
    }
}

fn refresh_view_model(
    engine: &EngineResource,
    view: &mut GameViewModel,
) -> Result<(), application::AppError> {
    let summary = engine.0.query(GameSummaryQuery);
    if view.summary.as_ref() != Some(&summary) {
        view.summary = Some(summary);
    }
    let battle_setup = engine.0.content_query(BattleSetupQuery);
    if view.battle_setup.as_ref() != Some(&battle_setup) {
        view.battle_setup = Some(battle_setup);
    }
    let save_slots = engine.0.list_save_slots()?;
    if view.save_slots != save_slots {
        view.save_slots = save_slots;
    }
    Ok(())
}

fn record_command_success(command: &Command, view: &mut GameViewModel) {
    let notice = match command {
        Command::CreatePet { .. } => Some("Pet adopted."),
        Command::ReleasePet { .. } => Some("Pet released."),
        Command::ResetGame { .. } => Some("Game reset. Adopt a new companion."),
        Command::UnlockAdoptionSlot => Some("Adoption slot unlocked."),
        Command::AdvanceTo { .. } => None,
        Command::FeedPet { .. } => Some("Pet fed successfully."),
        Command::Breed { .. } => Some("Egg created."),
        Command::HatchEgg { .. } => Some("Egg hatched."),
        Command::StartBattle { .. } => Some("Battle started."),
        Command::ResolveBattle { .. } => Some("Battle resolved."),
        Command::Save { .. } => Some("Game saved."),
        Command::Load { .. } => Some("Save slot loaded."),
    };
    if let Some(notice) = notice {
        view.last_error = None;
        view.last_notice = Some(notice.into());
    }
}

fn record_battle_result(
    mut events: MessageReader<'_, '_, DomainEventMessage>,
    mut view: ResMut<'_, GameViewModel>,
) {
    for message in events.read() {
        match &message.0 {
            domain::events::DomainEvent::WorldReset => view.last_battle_result = None,
            domain::events::DomainEvent::Combat(domain::combat::CombatEvent::BattleStarted {
                ..
            }) => view.last_battle_result = None,
            domain::events::DomainEvent::Combat(domain::combat::CombatEvent::BattleWon {
                winner,
                coins,
                experience,
                levels_gained,
                ..
            }) => {
                let Some(summary) = &view.summary else {
                    continue;
                };
                let Some(pet) = summary.pets.iter().find(|pet| pet.id == winner.to_string()) else {
                    continue;
                };
                view.last_battle_result = Some(BattleResultView {
                    winner_name: pet.name.clone(),
                    coins_earned: *coins,
                    balance: summary.balance,
                    experience_earned: *experience,
                    level: pet.level,
                    experience: pet.experience,
                    experience_for_next_level: pet.experience_for_next_level,
                    levels_gained: *levels_gained,
                });
            }
            _ => {}
        }
    }
}

fn record_errors(
    mut failures: MessageReader<'_, '_, CommandFailed>,
    mut view: ResMut<'_, GameViewModel>,
) {
    for failure in failures.read() {
        if failure.user_rejection {
            tracing::debug!(reason = %failure.message, "game command rejected");
            view.last_error = None;
            view.last_notice = Some(failure.message.clone());
        } else {
            tracing::error!(error = %failure.message, "game command failed");
            view.last_notice = None;
            view.last_error = Some(failure.message.clone());
        }
    }
}

fn enter_after_pet_roster_change(
    mut events: MessageReader<'_, '_, DomainEventMessage>,
    view: Res<'_, GameViewModel>,
    mut next: ResMut<'_, NextState<AppState>>,
    mut selection: ResMut<'_, PetSelection>,
) {
    for message in events.read() {
        match &message.0 {
            domain::events::DomainEvent::WorldReset => {
                *selection = PetSelection::default();
                next.set(AppState::PetCreation);
            }
            domain::events::DomainEvent::Pet(domain::events::PetEvent::Hatched { .. }) => {
                next.set(AppState::Home);
            }
            domain::events::DomainEvent::Pet(domain::events::PetEvent::Released { .. }) => {
                let empty = view
                    .summary
                    .as_ref()
                    .is_none_or(|summary| summary.pets.is_empty());
                next.set(if empty {
                    AppState::PetCreation
                } else {
                    AppState::Home
                });
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use domain::identifiers::SaveSlotId;
    use domain::rng::WorldSeed;
    use domain::time::Tick;

    use super::*;

    #[test]
    fn user_commands_replace_errors_with_success_feedback() {
        let mut view = GameViewModel {
            last_error: Some("previous failure".into()),
            ..default()
        };
        let slot = SaveSlotId::derive(WorldSeed(9), Tick::ZERO, 1);

        record_command_success(&Command::Save { slot }, &mut view);

        assert_eq!(view.last_error, None);
        assert_eq!(view.last_notice.as_deref(), Some("Game saved."));
    }

    #[test]
    fn background_clock_progress_does_not_erase_actionable_feedback() {
        let mut view = GameViewModel {
            last_error: Some("save failed".into()),
            last_notice: Some("older notice".into()),
            ..default()
        };

        record_command_success(
            &Command::AdvanceTo {
                wall_now: domain::time::UnixSeconds(60),
            },
            &mut view,
        );

        assert_eq!(view.last_error.as_deref(), Some("save failed"));
        assert_eq!(view.last_notice.as_deref(), Some("older notice"));
    }
}
