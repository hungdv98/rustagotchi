use std::path::{Path, PathBuf};

use application::ports::SaveRepository;
use application::{Command, GameEngine, GameState};
use bevy::a11y::AccessibilityNode;
use bevy::app::AppExit;
use bevy::asset::{AssetPlugin, RenderAssetUsages};
use bevy::image::ImagePlugin;
use bevy::input_focus::InputFocusPlugin;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::text::EditableText;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::Activate;
use content::ContentLoader;
use domain::genetics::{Ancestry, GeneticProfile, Parentage};
use domain::identifiers::{ItemId, PetId, SaveSlotId, SpeciesId};
use domain::pet::{LifeStage, Personality, Pet};
use domain::rng::WorldSeed;
use domain::time::{GameDuration, Tick, UnixSeconds};
use persistence::SqliteSaveRepository;
use presentation::{
    ActiveSaveSlot, AppCommand, AppState, EIGHT_PET_FRAME_BUDGET, EngineResource, GameViewModel,
    PresentationPreferences, RustagotchiPresentationPlugin,
};
use simulation::SimConfig;

fn assets() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

fn build_app(state: GameState, repository: SqliteSaveRepository, slot: SaveSlotId) -> App {
    let content = ContentLoader::load(assets()).unwrap();
    let config = SimConfig::from_ron(
        &std::fs::read_to_string(assets().join("config/simulation.ron")).unwrap(),
    )
    .unwrap();
    let saves: Box<dyn SaveRepository> = Box::new(repository);
    let engine = GameEngine::new(state, content, config, saves);
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin {
            file_path: assets().to_string_lossy().into_owned(),
            ..default()
        },
        ImagePlugin::default(),
        StatesPlugin,
        InputFocusPlugin,
    ))
    .insert_resource(EngineResource(engine))
    .insert_resource(ActiveSaveSlot(slot))
    .add_plugins(RustagotchiPresentationPlugin);
    pump(&mut app, 3);
    app
}

fn pump(app: &mut App, frames: usize) {
    for _ in 0..frames {
        app.update();
    }
}

fn current_state(app: &App) -> AppState {
    *app.world().resource::<State<AppState>>().get()
}

fn activate(app: &mut App, label: &str) {
    let entity = {
        let world = app.world_mut();
        let mut query = world.query::<(Entity, &AccessibilityNode)>();
        query
            .iter(world)
            .find_map(|(entity, node)| (node.label() == Some(label)).then_some(entity))
            .unwrap_or_else(|| panic!("button {label:?} was not present"))
    };
    app.world_mut().trigger(Activate { entity });
    pump(app, 4);
}

fn assert_button_disabled(app: &mut App, label: &str) {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &AccessibilityNode)>();
    let entity = query
        .iter(world)
        .find_map(|(entity, node)| (node.label() == Some(label)).then_some(entity))
        .unwrap_or_else(|| panic!("button {label:?} was not present"));
    assert!(world.entity(entity).contains::<InteractionDisabled>());
}

fn assert_text_contains(app: &mut App, expected: &str) {
    let world = app.world_mut();
    let mut query = world.query::<&Text>();
    assert!(
        query.iter(world).any(|text| text.contains(expected)),
        "screen did not contain {expected:?}"
    );
}

fn enter_pet_name(app: &mut App, name: &str) {
    let world = app.world_mut();
    let mut query = world.query::<&mut EditableText>();
    let mut input = query.single_mut(world).expect("pet-name input was present");
    *input = EditableText::new(name);
}

fn assert_pet_portrait_loaded(app: &mut App) {
    let asset_path = {
        let world = app.world_mut();
        let mut query = world.query::<&ImageNode>();
        let handle = query.single(world).unwrap().image.clone();
        world
            .resource::<AssetServer>()
            .get_path(handle.id())
            .unwrap()
            .to_string()
    };
    assert_eq!(asset_path, "sprites/emberling.png");

    let bytes = std::fs::read(assets().join(&asset_path)).unwrap();
    let image = Image::from_buffer(
        &bytes,
        bevy::image::ImageType::Extension("png"),
        bevy::image::CompressedImageFormats::NONE,
        true,
        bevy::image::ImageSampler::Default,
        RenderAssetUsages::default(),
    )
    .unwrap();
    assert_eq!(image.width(), 1_254);
    assert_eq!(image.height(), 1_254);
}

fn adult_founder(seed: WorldSeed, ordinal: u64) -> Pet {
    let mut pet = Pet::hatch(
        PetId::derive(seed, Tick::ZERO, ordinal),
        SpeciesId::new("emberling").unwrap(),
        GeneticProfile::new(),
        Personality::neutral(),
        Parentage::Founder,
        Ancestry::founder(),
        Tick::ZERO,
    );
    pet.stage = LifeStage::Adult;
    pet
}

#[test]
fn adoption_offers_every_species_shows_capacity_and_confirms_release() {
    let temporary = tempfile::tempdir().unwrap();
    let repository = SqliteSaveRepository::new(temporary.path().join("adoption.sqlite3")).unwrap();
    let seed = WorldSeed(76);
    let slot = SaveSlotId::derive(seed, Tick::ZERO, 1);
    let mut app = build_app(
        GameState::new(seed, UnixSeconds::EPOCH),
        repository.clone(),
        slot,
    );

    activate(&mut app, "Continue");
    assert_text_contains(&mut app, "Adopted 0/2");
    enter_pet_name(&mut app, "Ripple");
    activate(&mut app, "Aquafin");

    activate(&mut app, "Battle");
    activate(&mut app, "Adopt companion");
    assert_text_contains(&mut app, "Adopted 1/2");
    enter_pet_name(&mut app, "Volt");
    activate(&mut app, "Stormling");

    let species = app
        .world()
        .resource::<GameViewModel>()
        .summary
        .as_ref()
        .unwrap()
        .pets
        .iter()
        .map(|pet| pet.species.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        species,
        std::collections::BTreeSet::from(["aquafin", "stormling"])
    );

    activate(&mut app, "Status");
    activate(&mut app, "Release");
    assert_eq!(current_state(&app), AppState::PetRelease);
    assert_text_contains(&mut app, "cannot be undone");
    activate(&mut app, "Keep pet");
    assert_eq!(current_state(&app), AppState::PetStatus);
    activate(&mut app, "Release");
    activate(&mut app, "Confirm release");
    assert_eq!(current_state(&app), AppState::Home);
    let summary = app
        .world()
        .resource::<GameViewModel>()
        .summary
        .as_ref()
        .unwrap();
    assert_eq!(summary.pets.len(), 1);
    assert_eq!(summary.adopted_pets, 1);
    assert_eq!(summary.max_adopted_pets, 2);

    app.world_mut().write_message(AppExit::Success);
    pump(&mut app, 3);
    drop(app);
    let saved = repository.load(slot).unwrap();
    assert_eq!(saved.simulation.pets.len(), 1);

    let mut app = build_app(saved, repository.clone(), slot);
    activate(&mut app, "Reset game");
    assert_eq!(current_state(&app), AppState::ResetConfirmation);
    assert_text_contains(&mut app, "permanently erased");
    activate(&mut app, "Cancel");
    assert_eq!(current_state(&app), AppState::MainMenu);
    activate(&mut app, "Reset game");
    activate(&mut app, "Confirm reset");
    assert_eq!(current_state(&app), AppState::PetCreation);
    let summary = app
        .world()
        .resource::<GameViewModel>()
        .summary
        .as_ref()
        .unwrap();
    assert!(summary.pets.is_empty());
    assert!(summary.inventory.is_empty());
    assert!(matches!(
        repository.load(slot),
        Err(application::SaveError::NotFound)
    ));
}

#[test]
fn headless_ui_flow_cares_battles_saves_and_reloads() {
    let temporary = tempfile::tempdir().unwrap();
    let repository = SqliteSaveRepository::new(temporary.path().join("smoke.sqlite3")).unwrap();
    let seed = WorldSeed(77);
    let slot = SaveSlotId::derive(seed, Tick::ZERO, 1);
    let mut app = build_app(
        GameState::new(seed, UnixSeconds::EPOCH),
        repository.clone(),
        slot,
    );

    assert_eq!(current_state(&app), AppState::MainMenu);
    activate(&mut app, "Sound: On");
    assert!(
        !app.world()
            .resource::<PresentationPreferences>()
            .audio_enabled
    );
    activate(&mut app, "Motion: Full");
    assert!(
        app.world()
            .resource::<PresentationPreferences>()
            .reduced_motion
    );
    activate(&mut app, "Continue");
    assert_eq!(current_state(&app), AppState::PetCreation);
    enter_pet_name(&mut app, "Nova");
    activate(&mut app, "Emberling");
    assert_eq!(current_state(&app), AppState::Home);
    assert_pet_portrait_loaded(&mut app);
    assert_eq!(
        app.world()
            .resource::<GameViewModel>()
            .summary
            .as_ref()
            .unwrap()
            .pets[0]
            .name,
        "Nova"
    );
    assert_text_contains(&mut app, "Nova");

    activate(&mut app, "Status");
    assert_button_disabled(&mut app, "Pet is full, No food needed");
    assert_button_disabled(&mut app, "Only one pet");
    activate(&mut app, "Home");

    app.world_mut()
        .write_message(AppCommand(Command::AdvanceTo {
            wall_now: UnixSeconds(3_600),
        }));
    pump(&mut app, 2);

    activate(&mut app, "Status");
    activate(&mut app, "Feed apple, 5 slices available");
    let view = app.world().resource::<GameViewModel>();
    assert_eq!(view.summary.as_ref().unwrap().pets.len(), 1);
    assert_eq!(view.summary.as_ref().unwrap().inventory[0].quantity, 4);
    assert_eq!(
        view.last_notice.as_deref(),
        Some("Nova enjoyed an apple. 4 slices remain.")
    );
    assert_button_disabled(&mut app, "Pet is full, No food needed");

    let pet = app
        .world()
        .resource::<GameViewModel>()
        .summary
        .as_ref()
        .unwrap()
        .pets[0]
        .id
        .parse::<PetId>()
        .unwrap();
    app.world_mut().write_message(AppCommand(Command::FeedPet {
        pet,
        item: ItemId::new("apple_slice").unwrap(),
    }));
    pump(&mut app, 2);
    let view = app.world().resource::<GameViewModel>();
    assert_eq!(view.last_error, None);
    assert_eq!(
        view.last_notice.as_deref(),
        Some("No feeding needed - your pet is already full.")
    );

    activate(&mut app, "Inventory");
    activate(&mut app, "Home");
    activate(&mut app, "Battle");
    activate(&mut app, "Adopt companion");
    enter_pet_name(&mut app, "Bolt");
    activate(&mut app, "Emberling");
    assert_eq!(current_state(&app), AppState::Home);

    activate(&mut app, "Status");
    assert_text_contains(&mut app, "Pet 1 of 2");
    activate(&mut app, "Next pet");
    assert_text_contains(&mut app, "Pet 2 of 2");
    activate(&mut app, "Home");

    activate(&mut app, "Breed");
    assert_button_disabled(&mut app, "Parents need 7 days");
    let parent_ids = app
        .world()
        .resource::<GameViewModel>()
        .summary
        .as_ref()
        .unwrap()
        .pets
        .iter()
        .map(|pet| pet.id.parse::<PetId>().unwrap())
        .collect::<Vec<_>>();
    app.world_mut().write_message(AppCommand(Command::Breed {
        first: parent_ids[0],
        second: parent_ids[1],
    }));
    pump(&mut app, 2);
    let view = app.world().resource::<GameViewModel>();
    assert_eq!(view.last_error, None);
    assert_eq!(
        view.last_notice.as_deref(),
        Some("Both parents must be healthy adults at least 7 days old.")
    );
    activate(&mut app, "Home");

    activate(&mut app, "Battle");
    activate(&mut app, "Start battle");
    activate(&mut app, "Power");
    let expected_progress = {
        let view = app.world().resource::<GameViewModel>();
        assert!(view.summary.as_ref().unwrap().active_battles.is_empty());
        let result = view.last_battle_result.as_ref().unwrap();
        assert!(result.winner_name == "Nova" || result.winner_name == "Bolt");
        assert_eq!(result.coins_earned, 50);
        assert_eq!(result.experience_earned, 100);
        assert!(result.level >= 2);
        assert_eq!(view.last_notice, None);
        format!(
            "Level {} progress: {}/{} XP",
            result.level, result.experience, result.experience_for_next_level
        )
    };
    assert_text_contains(&mut app, "VICTORY -");
    assert_text_contains(&mut app, &expected_progress);
    activate(&mut app, "Home");
    assert_eq!(
        app.world().resource::<GameViewModel>().last_notice,
        None,
        "battle completion feedback must not leak onto Home"
    );

    app.world_mut().write_message(AppExit::Success);
    pump(&mut app, 3);
    assert_eq!(app.world().resource::<GameViewModel>().save_slots.len(), 1);
    drop(app);

    let saved = repository.load(slot).unwrap();
    let mut app = build_app(saved, repository, slot);
    let summary = app
        .world()
        .resource::<GameViewModel>()
        .summary
        .as_ref()
        .unwrap();
    assert_eq!(summary.pets.len(), 2);
    let mut names = summary
        .pets
        .iter()
        .map(|pet| pet.name.as_str())
        .collect::<Vec<_>>();
    names.sort_unstable();
    assert_eq!(names, ["Bolt", "Nova"]);
    assert_eq!(summary.inventory[0].quantity, 4);
    assert!(summary.active_battles.is_empty());

    app.world_mut()
        .write_message(AppCommand(Command::AdvanceTo {
            wall_now: UnixSeconds(7_200),
        }));
    pump(&mut app, 2);
    assert_eq!(
        app.world()
            .resource::<GameViewModel>()
            .summary
            .as_ref()
            .unwrap()
            .tick,
        120
    );
}

#[test]
fn coins_unlock_and_persist_an_adoption_slot_through_the_real_ui() {
    let temporary = tempfile::tempdir().unwrap();
    let repository = SqliteSaveRepository::new(temporary.path().join("economy.sqlite3")).unwrap();
    let seed = WorldSeed(33);
    let slot = SaveSlotId::derive(seed, Tick::ZERO, 1);
    let mut state = GameState::new(seed, UnixSeconds::EPOCH);
    state.simulation.insert_pet(adult_founder(seed, 1));
    state.wallet = domain::economy::Wallet::with_balance(domain::economy::Currency(150));
    state.next_entity_ordinal = 2;
    let mut app = build_app(state, repository.clone(), slot);

    activate(&mut app, "Continue");
    activate(&mut app, "Adopt");
    assert_text_contains(&mut app, "Adopted 1/2  |  Wallet 150 coins");
    activate(&mut app, "Unlock 3/8 - 100 coins");
    let summary = app
        .world()
        .resource::<GameViewModel>()
        .summary
        .as_ref()
        .unwrap();
    assert_eq!(summary.balance, 50);
    assert_eq!(summary.max_adopted_pets, 3);
    assert_eq!(summary.next_adoption_slot_cost, Some(200));

    app.world_mut().write_message(AppExit::Success);
    pump(&mut app, 3);
    drop(app);
    let saved = repository.load(slot).unwrap();
    assert_eq!(saved.adoption_slot_upgrades, 1);
    assert_eq!(saved.wallet.balance().0, 50);
}

#[test]
fn headless_ui_flow_breeds_incubates_hatches_and_reloads() {
    let temporary = tempfile::tempdir().unwrap();
    let repository = SqliteSaveRepository::new(temporary.path().join("breeding.sqlite3")).unwrap();
    let seed = WorldSeed(1);
    let slot = SaveSlotId::derive(seed, Tick::ZERO, 1);
    let adult_age = GameDuration::from_days(7);
    let mut state = GameState::new(seed, UnixSeconds::EPOCH);
    state.simulation.tick = Tick::ZERO.saturating_add(adult_age);
    state.clock.advance(adult_age);
    state.simulation.insert_pet(adult_founder(seed, 1));
    state.simulation.insert_pet(adult_founder(seed, 2));
    state.next_entity_ordinal = 3;
    let mut app = build_app(state, repository.clone(), slot);

    activate(&mut app, "Continue");
    activate(&mut app, "Breed");
    activate(&mut app, "Breed selected pair");
    assert_eq!(
        app.world()
            .resource::<GameViewModel>()
            .summary
            .as_ref()
            .unwrap()
            .eggs
            .len(),
        1
    );
    assert_text_contains(&mut app, "2d 0h remaining | 0% incubated");
    assert_button_disabled(&mut app, "Egg is incubating");

    app.world_mut()
        .write_message(AppCommand(Command::AdvanceTo {
            wall_now: UnixSeconds(9 * 86_400),
        }));
    pump(&mut app, 3);
    assert_text_contains(&mut app, "Ready to hatch | 100% incubated");
    activate(&mut app, "Hatch egg");
    let summary = app
        .world()
        .resource::<GameViewModel>()
        .summary
        .as_ref()
        .unwrap();
    assert!(summary.eggs.is_empty());
    assert_eq!(summary.pets.len(), 3);

    app.world_mut().write_message(AppExit::Success);
    pump(&mut app, 3);
    drop(app);

    let saved = repository.load(slot).unwrap();
    assert_eq!(saved.simulation.pets.len(), 3);
    assert!(saved.eggs.is_empty());
}

#[test]
fn eight_pet_headless_presentation_stays_inside_cpu_frame_budget() {
    let temporary = tempfile::tempdir().unwrap();
    let repository =
        SqliteSaveRepository::new(temporary.path().join("performance.sqlite3")).unwrap();
    let seed = WorldSeed(88);
    let slot = SaveSlotId::derive(seed, Tick::ZERO, 1);
    let mut state = GameState::new(seed, UnixSeconds::EPOCH);
    for ordinal in 1..=8 {
        state.simulation.insert_pet(adult_founder(seed, ordinal));
    }
    state.next_entity_ordinal = 9;
    let mut app = build_app(state, repository, slot);

    activate(&mut app, "Continue");
    assert_eq!(current_state(&app), AppState::Home);
    let portrait_count = {
        let world = app.world_mut();
        let mut query = world.query::<&ImageNode>();
        query.iter(world).count()
    };
    assert_eq!(portrait_count, 8);

    pump(&mut app, 120);
    let frame_count = 1_000_u32;
    let started = std::time::Instant::now();
    pump(&mut app, frame_count as usize);
    let average = started.elapsed() / frame_count;
    eprintln!("eight-pet headless average CPU frame: {average:?}");

    assert!(
        average <= EIGHT_PET_FRAME_BUDGET,
        "eight-pet headless CPU frame {average:?} exceeded {:?}",
        EIGHT_PET_FRAME_BUDGET
    );
}
