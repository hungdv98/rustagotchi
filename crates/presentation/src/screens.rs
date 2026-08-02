use accesskit::{Node as AccessibilityTreeNode, Role};
use bevy::a11y::AccessibilityNode;
use bevy::ecs::spawn::SpawnIter;
use bevy::ecs::system::SystemParam;
use bevy::input_focus::tab_navigation::{TabGroup, TabIndex};
use bevy::input_focus::{AutoFocus, InputFocus, InputFocusVisible};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::text::{EditableText, EditableTextFilter, TextCursorStyle};
use bevy::ui::{InteractionDisabled, Pressed};
use bevy::ui_widgets::Activate;

use application::Command;
use domain::combat::BattleAction;
use domain::identifiers::{BattleId, EggId, ItemId, MoveId, PetId, SpeciesId};
use domain::pet::{MAX_PET_NAME_CHARS, PetName};

use crate::animation::PetAnimator;
use crate::{
    ActiveSaveSlot, AppCommand, AppState, CommandFailed, GameViewModel, PetSelection,
    PresentationPreferences, SwitchSaveSlot,
};

#[derive(Component, Debug)]
#[require(TabGroup)]
pub(crate) struct ScreenRoot;

#[derive(Clone, Copy, Component, Debug, Eq, PartialEq)]
pub(crate) enum NavigateTo {
    Continue,
    MainMenu,
    Save,
    RequestReset,
    ConfirmReset,
    UnlockAdoptionSlot,
    AdoptEmberling,
    AdoptStormling,
    AdoptAquafin,
    PetStatus,
    Inventory,
    FeedSelectedPet,
    CyclePrimary,
    CycleSecondary,
    RequestRelease,
    ConfirmRelease,
    Breeding,
    AddCompanion,
    BreedSelectedPair,
    HatchFirstEgg,
    Noop,
    Battle,
    StartSelectedBattle,
    ResolveBattlePower,
    ResolveBattlePrecision,
    ResolveBattleEfficient,
    SaveSlots,
    LoadNextSlot,
    ToggleAudio,
    ToggleReducedMotion,
}

impl NavigateTo {
    const fn changes_screen(self) -> bool {
        matches!(
            self,
            Self::Continue
                | Self::MainMenu
                | Self::RequestReset
                | Self::PetStatus
                | Self::Inventory
                | Self::RequestRelease
                | Self::Breeding
                | Self::AddCompanion
                | Self::Battle
                | Self::SaveSlots
        )
    }
}

#[derive(Clone, Copy, Debug, Message)]
pub(crate) struct MenuActivated(NavigateTo);

#[derive(Component, Debug)]
pub(crate) struct PetStatusDetails;

#[derive(Component, Debug)]
pub(crate) struct PetStatusPortrait;

#[derive(Component, Debug)]
pub(crate) struct PetNeedFill(usize);

#[derive(Component, Debug)]
pub(crate) struct PetNeedValue(usize);

#[derive(Component, Debug)]
pub(crate) struct CareActionButton;

#[derive(Component, Debug)]
pub(crate) struct CareActionTitle;

#[derive(Component, Debug)]
pub(crate) struct CareActionDetail;

#[derive(Component, Debug)]
pub(crate) struct BreedingActionButton;

#[derive(Component, Debug)]
pub(crate) struct PetCycleButton;

#[derive(Component, Debug)]
pub(crate) struct InventoryDetails;

#[derive(Component, Debug)]
pub(crate) struct SaveSlotDetails;

#[derive(Component, Debug)]
pub(crate) struct HomeDetails;

#[derive(Component, Debug)]
pub(crate) struct StatusDetails;

#[derive(Component, Debug)]
pub(crate) struct PreferenceDetails;

#[derive(Component, Debug)]
pub(crate) struct PetNameInput;

#[derive(Component, Debug)]
pub(crate) struct AudioPreferenceButton;

#[derive(Component, Debug)]
pub(crate) struct MotionPreferenceButton;

#[derive(SystemParam)]
pub(crate) struct NavigationIo<'w, 's> {
    name_inputs: Query<'w, 's, &'static EditableText, With<PetNameInput>>,
    commands: MessageWriter<'w, AppCommand>,
    failures: MessageWriter<'w, CommandFailed>,
    switches: MessageWriter<'w, SwitchSaveSlot>,
}

#[derive(Component, Debug)]
pub(crate) struct PetPortrait(usize);

#[derive(Component, Debug)]
pub(crate) struct PetNameLabel(usize);

#[derive(Component, Debug)]
pub(crate) struct PetSpeciesLabel(usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BattleTactic {
    Power,
    Precision,
    Efficient,
}

pub(crate) fn spawn_main_menu(
    mut commands: Commands<'_, '_>,
    view: Res<'_, GameViewModel>,
    preferences: Res<'_, PresentationPreferences>,
) {
    commands.spawn((
        ScreenRoot,
        screen_node(),
        BackgroundColor(Color::srgb(0.055, 0.07, 0.10)),
        children![
            heading("Rustagotchi"),
            (
                Text::new("A deterministic virtual-pet world"),
                content_node(),
                TextLayout::justify(Justify::Center),
                TextFont::from_font_size(16.0),
                TextColor(Color::srgb(0.78, 0.82, 0.88)),
            ),
            (
                Text::new(preference_text(*preferences)),
                PreferenceDetails,
                content_node(),
                TextLayout::justify(Justify::Center),
                TextFont::from_font_size(16.0),
                TextColor(Color::srgb(0.67, 0.73, 0.80)),
            ),
            status_banner(&view),
            menu_button("Continue", NavigateTo::Continue),
            menu_button("Save slots", NavigateTo::SaveSlots),
            menu_button("Reset game", NavigateTo::RequestReset),
            (
                menu_button(
                    preference_button_label(*preferences, true),
                    NavigateTo::ToggleAudio,
                ),
                AudioPreferenceButton,
            ),
            (
                menu_button(
                    preference_button_label(*preferences, false),
                    NavigateTo::ToggleReducedMotion,
                ),
                MotionPreferenceButton,
            ),
        ],
    ));
}

pub(crate) fn spawn_reset_confirmation(
    mut commands: Commands<'_, '_>,
    view: Res<'_, GameViewModel>,
) {
    commands.spawn((
        ScreenRoot,
        screen_node(),
        BackgroundColor(Color::srgb(0.055, 0.07, 0.10)),
        children![
            heading("Reset game?"),
            (
                Text::new("All pets, eggs, inventory, coins and battle progress in this save slot will be permanently erased."),
                content_node(),
                TextLayout::justify(Justify::Center),
                TextFont::from_font_size(18.0),
                TextColor(Color::srgb(0.95, 0.62, 0.46)),
            ),
            status_banner(&view),
            menu_button("Confirm reset", NavigateTo::ConfirmReset),
            menu_button("Cancel", NavigateTo::MainMenu),
        ],
    ));
}

pub(crate) fn spawn_save_slots(
    mut commands: Commands<'_, '_>,
    view: Res<'_, GameViewModel>,
    active: Res<'_, ActiveSaveSlot>,
) {
    commands.spawn((
        ScreenRoot,
        screen_node(),
        BackgroundColor(Color::srgb(0.055, 0.07, 0.10)),
        children![
            heading("Save slots"),
            (
                Text::new(save_slots_text(&view, active.0)),
                SaveSlotDetails,
                content_node(),
                TextLayout::justify(Justify::Center),
                TextFont::from_font_size(18.0),
                TextColor(Color::srgb(0.78, 0.82, 0.88)),
            ),
            status_banner(&view),
            menu_button("Save current", NavigateTo::Save),
            menu_button("Load next", NavigateTo::LoadNextSlot),
            menu_button("Back", NavigateTo::MainMenu),
        ],
    ));
}

pub(crate) fn spawn_pet_creation(mut commands: Commands<'_, '_>, view: Res<'_, GameViewModel>) {
    let (adopted, maximum, balance) = adoption_summary(&view);
    commands.spawn((
        ScreenRoot,
        screen_node(),
        BackgroundColor(Color::srgb(0.055, 0.07, 0.10)),
        children![
            heading("Adopt a pet"),
            (
                Text::new(format!(
                    "Adopted {adopted}/{maximum}  |  Wallet {balance} coins"
                )),
                content_node(),
                TextLayout::justify(Justify::Center),
                TextFont::from_font_size(19.0),
                TextColor(Color::srgb(0.78, 0.82, 0.88)),
            ),
            pet_name_input(),
            (
                Text::new(format!(
                    "1-{MAX_PET_NAME_CHARS} letters, numbers, spaces, - or '"
                )),
                TextFont::from_font_size(14.0),
                TextColor(Color::srgb(0.58, 0.65, 0.72)),
            ),
            status_banner(&view),
            adoption_upgrade_button(&view),
            adoption_choices(),
            menu_button("Home", NavigateTo::Continue),
        ],
    ));
}

fn adoption_summary(view: &GameViewModel) -> (usize, usize, i64) {
    view.summary
        .as_ref()
        .map_or((0, application::MAX_ADOPTED_PETS, 0), |summary| {
            (
                summary.adopted_pets,
                summary.max_adopted_pets,
                summary.balance,
            )
        })
}

fn adoption_choices() -> impl Bundle + use<> {
    (
        Node {
            width: percent(100),
            max_width: px(620),
            flex_direction: FlexDirection::Row,
            column_gap: px(8),
            justify_content: JustifyContent::Center,
            ..default()
        },
        children![
            compact_menu_button("Emberling", NavigateTo::AdoptEmberling),
            compact_menu_button("Stormling", NavigateTo::AdoptStormling),
            compact_menu_button("Aquafin", NavigateTo::AdoptAquafin),
        ],
    )
}

fn adoption_upgrade_button(view: &GameViewModel) -> impl Bundle + use<> {
    let (label, action) = view.summary.as_ref().map_or_else(
        || ("Slots unavailable".to_owned(), NavigateTo::Noop),
        |summary| match summary.next_adoption_slot_cost {
            Some(cost) => {
                let verb = if summary.balance >= cost {
                    "Unlock"
                } else {
                    "Save for"
                };
                (
                    format!(
                        "{verb} {}/{} - {cost} coins",
                        summary.max_adopted_pets + 1,
                        summary.maximum_adopted_pets,
                    ),
                    NavigateTo::UnlockAdoptionSlot,
                )
            }
            None => (
                format!(
                    "All {} adoption slots unlocked",
                    summary.maximum_adopted_pets
                ),
                NavigateTo::Noop,
            ),
        },
    );
    let mut accessibility = AccessibilityTreeNode::new(Role::Button);
    accessibility.set_label(label.clone());
    (
        bevy::ui_widgets::Button,
        action,
        TabIndex(0),
        Hovered::default(),
        AccessibilityNode(accessibility),
        Node {
            width: percent(100),
            max_width: px(360),
            min_height: px(54),
            border: UiRect::all(px(3)),
            padding: UiRect::axes(px(10), px(8)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border_radius: BorderRadius::all(px(12)),
            ..default()
        },
        BorderColor::all(Color::NONE),
        BackgroundColor(Color::srgb(0.16, 0.25, 0.32)),
        children![(
            Text::new(label),
            TextLayout::no_wrap(),
            TextFont::from_font_size(18.0),
            TextColor(Color::WHITE),
        )],
    )
}

fn pet_name_input() -> impl Bundle + use<> {
    let mut accessibility = AccessibilityTreeNode::new(Role::TextInput);
    accessibility.set_label("Pet name");
    (
        EditableText::new("Sparky"),
        EditableTextFilter::new(|character| {
            character.is_alphanumeric() || matches!(character, ' ' | '-' | '\'')
        }),
        PetNameInput,
        AutoFocus,
        TabIndex(0),
        TextLayout::no_wrap(),
        TextCursorStyle {
            color: Color::srgb(0.95, 0.78, 0.38),
            selected_text_color: Some(Color::BLACK),
            ..default()
        },
        TextFont::from_font_size(24.0),
        TextColor(Color::WHITE),
        AccessibilityNode(accessibility),
        Node {
            width: percent(100),
            max_width: px(320),
            min_height: px(54),
            border: UiRect::all(px(2)),
            padding: UiRect::axes(px(14), px(10)),
            ..default()
        },
        BorderColor::all(Color::srgb(0.31, 0.48, 0.60)),
        BackgroundColor(Color::srgb(0.09, 0.13, 0.18)),
    )
}

pub(crate) fn spawn_home(
    mut commands: Commands<'_, '_>,
    view: Res<'_, GameViewModel>,
    asset_server: Res<'_, AssetServer>,
) {
    commands.spawn((
        ScreenRoot,
        home_screen_node(),
        BackgroundColor(Color::srgb(0.055, 0.07, 0.10)),
        children![
            heading("Your pets"),
            (
                Text::new(home_text(&view)),
                HomeDetails,
                content_node(),
                TextLayout::justify(Justify::Center),
                TextFont::from_font_size(19.0),
                TextColor(Color::srgb(0.78, 0.82, 0.88)),
            ),
            pet_gallery(&asset_server, &view),
            status_banner(&view),
            home_navigation(),
        ],
    ));
}

fn home_screen_node() -> Node {
    Node {
        width: percent(100),
        height: percent(100),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        row_gap: px(12),
        padding: UiRect::all(px(16)),
        overflow: Overflow::clip(),
        ..default()
    }
}

fn home_navigation() -> impl Bundle + use<> {
    (
        Node {
            width: percent(100),
            max_width: px(540),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::Center,
            column_gap: px(12),
            ..default()
        },
        children![
            compact_menu_button("Adopt", NavigateTo::AddCompanion),
            compact_menu_button("Status", NavigateTo::PetStatus),
            compact_menu_button("Breed", NavigateTo::Breeding),
            compact_menu_button("Battle", NavigateTo::Battle),
            compact_menu_button("Menu", NavigateTo::MainMenu),
        ],
    )
}

fn pet_gallery(asset_server: &AssetServer, view: &GameViewModel) -> impl Bundle + use<> {
    let pets: Vec<_> = view
        .summary
        .as_ref()
        .into_iter()
        .flat_map(|summary| summary.pets.iter())
        .take(8)
        .enumerate()
        .map(|(index, pet)| {
            (
                index,
                pet.name.clone(),
                pet.species.clone(),
                asset_server.load(sprite_path(&pet.species)),
            )
        })
        .collect();
    let size = if pets.len() == 1 { 132.0 } else { 62.0 };
    let rows = pets.len().div_ceil(4).max(1) as f32;
    let gallery_height = rows * (size + 34.0) + (rows - 1.0) * 8.0;
    (
        Node {
            width: percent(100),
            max_width: px(420),
            height: px(gallery_height),
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            column_gap: px(8),
            row_gap: px(6),
            ..default()
        },
        Children::spawn(SpawnIter(pets.into_iter().map(
            move |(index, name, species, image)| pet_portrait(image, index, name, species, size),
        ))),
    )
}

fn sprite_path(species: &str) -> &'static str {
    match species {
        "stormling" => "sprites/stormling.png",
        "aquafin" => "sprites/aquafin.png",
        _ => "sprites/emberling.png",
    }
}

fn pet_portrait(
    image: Handle<Image>,
    index: usize,
    name: String,
    species: String,
    size: f32,
) -> impl Bundle + use<> {
    let mut accessibility = AccessibilityTreeNode::new(Role::Image);
    accessibility.set_label(format!("{name}, {species}, pet {}", index + 1));
    (
        Node {
            width: px(size + 12.0),
            height: px(size + 34.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            flex_shrink: 0.0,
            ..default()
        },
        children![
            (
                ImageNode::new(image),
                PetPortrait(index),
                PetAnimator::new(index),
                UiTransform::IDENTITY,
                Node {
                    width: px(size),
                    height: px(size),
                    flex_shrink: 0.0,
                    ..default()
                },
                AccessibilityNode(accessibility),
            ),
            (
                Text::new(name),
                PetNameLabel(index),
                TextFont::from_font_size(if size > 100.0 { 19.0 } else { 14.0 }),
                TextColor(Color::srgb(0.95, 0.78, 0.38)),
            ),
            (
                Text::new(display_name(&species)),
                PetSpeciesLabel(index),
                TextFont::from_font_size(if size > 100.0 { 14.0 } else { 11.0 }),
                TextColor(Color::srgb(0.62, 0.69, 0.76)),
            ),
        ],
    )
}

pub(crate) fn spawn_pet_status(
    mut commands: Commands<'_, '_>,
    view: Res<'_, GameViewModel>,
    selection: Res<'_, PetSelection>,
    asset_server: Res<'_, AssetServer>,
) {
    let (care_title, care_detail, care_action, _) = care_action(&view, *selection);
    commands.spawn((
        ScreenRoot,
        pet_status_screen_node(),
        BackgroundColor(Color::srgb(0.055, 0.07, 0.10)),
        children![
            heading("Pet status"),
            pet_status_hero(&asset_server, &view, *selection),
            need_panel(&view, *selection),
            status_banner(&view),
            care_action_button(&care_title, &care_detail, care_action),
            pet_status_navigation(&view),
        ],
    ));
}

fn care_action_button(title: &str, detail: &str, action: NavigateTo) -> impl Bundle + use<> {
    let mut accessibility = AccessibilityTreeNode::new(Role::Button);
    accessibility.set_label(format!("{title}, {detail}"));
    (
        bevy::ui_widgets::Button,
        action,
        CareActionButton,
        TabIndex(0),
        Hovered::default(),
        AccessibilityNode(accessibility),
        Node {
            width: percent(100),
            max_width: px(360),
            min_height: px(68),
            flex_direction: FlexDirection::Column,
            row_gap: px(2),
            border: UiRect::all(px(3)),
            padding: UiRect::axes(px(12), px(7)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border_radius: BorderRadius::all(px(12)),
            ..default()
        },
        BorderColor::all(Color::NONE),
        BackgroundColor(Color::srgb(0.16, 0.25, 0.32)),
        children![
            (
                Text::new(title.to_owned()),
                CareActionTitle,
                TextFont::from_font_size(20.0),
                TextColor(Color::WHITE),
            ),
            (
                Text::new(detail.to_owned()),
                CareActionDetail,
                TextFont::from_font_size(14.0),
                TextColor(Color::srgb(0.70, 0.78, 0.84)),
            ),
        ],
    )
}

fn pet_status_hero(
    asset_server: &AssetServer,
    view: &GameViewModel,
    selection: PetSelection,
) -> impl Bundle + use<> {
    let pet = selected_pet(view, selection);
    let species = pet.map_or("emberling", |pet| pet.species.as_str());
    let label = pet.map_or_else(
        || "No selected pet".to_owned(),
        |pet| format!("{}, {} portrait", pet.name, display_name(&pet.species)),
    );
    let mut accessibility = AccessibilityTreeNode::new(Role::Image);
    accessibility.set_label(label);
    (
        Node {
            width: percent(100),
            max_width: px(620),
            min_height: px(132),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(18),
            padding: UiRect::all(px(12)),
            border_radius: BorderRadius::all(px(16)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.075, 0.10, 0.14)),
        children![
            (
                ImageNode::new(asset_server.load(sprite_path(species))),
                PetStatusPortrait,
                AccessibilityNode(accessibility),
                Node {
                    width: px(108),
                    height: px(108),
                    flex_shrink: 0.0,
                    ..default()
                },
            ),
            (
                Text::new(pet_status_text(view, selection)),
                PetStatusDetails,
                Node {
                    flex_grow: 1.0,
                    ..default()
                },
                TextLayout::justify(Justify::Left),
                TextFont::from_font_size(18.0),
                TextColor(Color::srgb(0.82, 0.86, 0.92)),
            ),
        ],
    )
}

pub(crate) fn spawn_pet_release(
    mut commands: Commands<'_, '_>,
    view: Res<'_, GameViewModel>,
    selection: Res<'_, PetSelection>,
) {
    let name = selected_pet(&view, *selection).map_or("this pet", |pet| pet.name.as_str());
    commands.spawn((
        ScreenRoot,
        screen_node(),
        BackgroundColor(Color::srgb(0.055, 0.07, 0.10)),
        children![
            heading("Release pet?"),
            (
                Text::new(format!(
                    "Release {name} permanently? This cannot be undone."
                )),
                content_node(),
                TextLayout::justify(Justify::Center),
                TextFont::from_font_size(19.0),
                TextColor(Color::srgb(0.92, 0.68, 0.48)),
            ),
            status_banner(&view),
            menu_button("Confirm release", NavigateTo::ConfirmRelease),
            menu_button("Keep pet", NavigateTo::PetStatus),
        ],
    ));
}

fn pet_status_screen_node() -> Node {
    Node {
        width: percent(100),
        height: percent(100),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        row_gap: px(10),
        padding: UiRect::all(px(12)),
        overflow: Overflow::clip(),
        ..default()
    }
}

fn need_panel(view: &GameViewModel, selection: PetSelection) -> impl Bundle + use<> {
    let needs = selected_pet(view, selection)
        .map(|pet| pet.needs.clone())
        .unwrap_or_default();
    (
        Node {
            width: percent(100),
            max_width: px(540),
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(8),
            row_gap: px(8),
            padding: UiRect::all(px(10)),
            border_radius: BorderRadius::all(px(12)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.085, 0.11, 0.15)),
        Children::spawn(SpawnIter(
            needs
                .into_iter()
                .enumerate()
                .map(|(index, need)| need_meter(index, need.kind, need.value_milli)),
        )),
    )
}

fn need_meter(index: usize, kind: String, value_milli: i32) -> impl Bundle + use<> {
    let percent_value = (value_milli / 1_000).clamp(0, 100);
    (
        Node {
            width: percent(48),
            min_height: px(48),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            row_gap: px(5),
            ..default()
        },
        children![
            (
                Text::new(need_value_text(&kind, value_milli)),
                PetNeedValue(index),
                TextFont::from_font_size(14.0),
                TextColor(Color::srgb(0.84, 0.87, 0.91)),
            ),
            (
                Node {
                    width: percent(100),
                    height: px(12),
                    border_radius: BorderRadius::all(px(6)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.12, 0.15, 0.19)),
                children![(
                    PetNeedFill(index),
                    Node {
                        width: percent(percent_value as f32),
                        height: percent(100),
                        border_radius: BorderRadius::all(px(6)),
                        ..default()
                    },
                    BackgroundColor(need_color(value_milli)),
                )],
            ),
        ],
    )
}

fn pet_status_navigation(view: &GameViewModel) -> impl Bundle + use<> {
    let (cycle_label, cycle_action, _) = pet_cycle_action(view);
    (
        Node {
            width: percent(100),
            max_width: px(540),
            flex_direction: FlexDirection::Row,
            column_gap: px(8),
            justify_content: JustifyContent::Center,
            ..default()
        },
        children![
            (
                compact_menu_button(&cycle_label, cycle_action),
                PetCycleButton,
            ),
            compact_menu_button("Inventory", NavigateTo::Inventory),
            compact_menu_button("Release", NavigateTo::RequestRelease),
            compact_menu_button("Home", NavigateTo::Continue),
        ],
    )
}

fn pet_cycle_action(view: &GameViewModel) -> (String, NavigateTo, bool) {
    if view
        .summary
        .as_ref()
        .is_some_and(|summary| summary.pets.len() > 1)
    {
        ("Next pet".into(), NavigateTo::CyclePrimary, false)
    } else {
        ("Only one pet".into(), NavigateTo::Noop, true)
    }
}

pub(crate) fn spawn_breeding(
    mut commands: Commands<'_, '_>,
    view: Res<'_, GameViewModel>,
    selection: Res<'_, PetSelection>,
) {
    commands.spawn((ScreenRoot, breeding_page(&view, *selection)));
}

pub(crate) fn spawn_battle(
    mut commands: Commands<'_, '_>,
    view: Res<'_, GameViewModel>,
    selection: Res<'_, PetSelection>,
    asset_server: Res<'_, AssetServer>,
) {
    commands.spawn((ScreenRoot, battle_page(&asset_server, &view, *selection)));
}

fn battle_page(
    asset_server: &AssetServer,
    view: &GameViewModel,
    selection: PetSelection,
) -> impl Bundle + use<> {
    let active = view
        .summary
        .as_ref()
        .is_some_and(|summary| !summary.active_battles.is_empty());
    let (primary_label, primary_action) = battle_primary_action(view);
    let buttons = if active {
        [
            ("Power", NavigateTo::ResolveBattlePower),
            ("Precision", NavigateTo::ResolveBattlePrecision),
            ("Efficiency", NavigateTo::ResolveBattleEfficient),
            ("Home", NavigateTo::Continue),
        ]
    } else {
        [
            ("Next A", NavigateTo::CyclePrimary),
            ("Next B", NavigateTo::CycleSecondary),
            (&primary_label, primary_action),
            ("Home", NavigateTo::Continue),
        ]
    };
    (
        battle_screen_node(),
        BackgroundColor(Color::srgb(0.045, 0.06, 0.09)),
        children![
            heading(if active {
                "Battle arena"
            } else if view.last_battle_result.is_some() {
                "Battle results"
            } else {
                "Choose fighters"
            }),
            battle_arena(asset_server, view, selection),
            battle_result_panel(view),
            (
                Text::new(battle_guidance(view, active)),
                content_node(),
                TextLayout::justify(Justify::Center),
                TextFont::from_font_size(16.0),
                TextColor(Color::srgb(0.72, 0.78, 0.86)),
            ),
            status_banner(view),
            battle_action_row(buttons),
        ],
    )
}

fn battle_arena(
    asset_server: &AssetServer,
    view: &GameViewModel,
    selection: PetSelection,
) -> impl Bundle + use<> {
    let active = view
        .summary
        .as_ref()
        .and_then(|summary| summary.active_battles.first());
    let fighters = view.battle_setup.as_ref().and_then(|setup| {
        if let Some(battle) = active {
            let first = setup
                .pets
                .iter()
                .find(|pet| pet.id == battle.combatant_ids[0])?;
            let second = setup
                .pets
                .iter()
                .find(|pet| pet.id == battle.combatant_ids[1])?;
            Some([first, second])
        } else {
            let indices = selection.pair(setup.pets.len())?;
            Some([&setup.pets[indices[0]], &setup.pets[indices[1]]])
        }
    });
    let first = fighters.map(|fighters| fighter_card(asset_server, fighters[0], active, 0));
    let second = fighters.map(|fighters| fighter_card(asset_server, fighters[1], active, 1));
    (
        Node {
            width: percent(100),
            max_width: px(680),
            min_height: px(270),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            column_gap: px(12),
            padding: UiRect::all(px(12)),
            border_radius: BorderRadius::all(px(20)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.07, 0.095, 0.13)),
        Children::spawn(SpawnIter(first.into_iter().chain(second))),
    )
}

fn fighter_card(
    asset_server: &AssetServer,
    pet: &application::dto::BattlePetDto,
    battle: Option<&application::dto::BattleDto>,
    index: usize,
) -> impl Bundle + use<> {
    let health = battle.map_or(100, |battle| battle.health[index]);
    let energy = battle.map_or(100, |battle| battle.energy[index]);
    let move_summary = pet
        .moves
        .iter()
        .map(|battle_move| display_name(&battle_move.id))
        .collect::<Vec<_>>()
        .join(" / ");
    let mut accessibility = AccessibilityTreeNode::new(Role::Image);
    accessibility.set_label(format!("{}, {} fighter", pet.name, pet.species));
    (
        Node {
            width: percent(48),
            max_width: px(300),
            min_height: px(238),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: px(5),
            padding: UiRect::all(px(10)),
            border: UiRect::all(px(2)),
            border_radius: BorderRadius::all(px(16)),
            ..default()
        },
        BorderColor::all(if index == 0 {
            Color::srgb(0.30, 0.68, 0.92)
        } else {
            Color::srgb(0.94, 0.48, 0.36)
        }),
        BackgroundColor(Color::srgb(0.09, 0.12, 0.17)),
        children![
            (
                ImageNode::new(asset_server.load(sprite_path(&pet.species))),
                AccessibilityNode(accessibility),
                Node {
                    width: px(112),
                    height: px(112),
                    ..default()
                },
            ),
            (
                Text::new(pet.name.clone()),
                TextFont::from_font_size(21.0),
                TextColor(Color::srgb(0.96, 0.80, 0.42)),
            ),
            (
                Text::new(format!(
                    "{}  |  HP {health}  |  EN {energy}",
                    display_name(&pet.species)
                )),
                TextFont::from_font_size(14.0),
                TextColor(Color::srgb(0.80, 0.85, 0.91)),
            ),
            (
                Text::new(move_summary),
                TextLayout::justify(Justify::Center),
                TextFont::from_font_size(12.0),
                TextColor(Color::srgb(0.58, 0.67, 0.77)),
            ),
        ],
    )
}

fn battle_guidance(view: &GameViewModel, active: bool) -> String {
    if active {
        "Choose Power for damage, Precision for accuracy, or Efficiency to conserve energy.".into()
    } else if view
        .summary
        .as_ref()
        .is_some_and(|summary| summary.pets.len() < 2)
    {
        "Adopt a second living pet to unlock the arena.".into()
    } else {
        "Select two fighters, compare their move pools, then start the match.".into()
    }
}

fn battle_result_panel(view: &GameViewModel) -> impl Bundle + use<> {
    let result = view.last_battle_result.as_ref();
    let text = result.map_or_else(String::new, |result| {
        let level_up = if result.levels_gained == 0 {
            String::new()
        } else {
            format!("  LEVEL UP +{}", result.levels_gained)
        };
        let percent = if result.experience_for_next_level == u32::MAX {
            100
        } else {
            result.experience.saturating_mul(100) / result.experience_for_next_level.max(1)
        };
        format!(
            "VICTORY - {}{level_up}\n+{} coins  |  Wallet: {}  |  +{} XP\nLevel {} progress: {}/{} XP ({percent}%)",
            result.winner_name,
            result.coins_earned,
            result.balance,
            result.experience_earned,
            result.level,
            result.experience,
            result.experience_for_next_level,
        )
    });
    (
        Node {
            display: if result.is_some() {
                Display::Flex
            } else {
                Display::None
            },
            width: percent(100),
            max_width: px(680),
            min_height: px(78),
            padding: UiRect::axes(px(14), px(9)),
            border: UiRect::all(px(2)),
            border_radius: BorderRadius::all(px(12)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BorderColor::all(Color::srgb(0.95, 0.70, 0.28)),
        BackgroundColor(Color::srgb(0.12, 0.105, 0.075)),
        children![(
            Text::new(text),
            TextLayout::justify(Justify::Center),
            TextFont::from_font_size(16.0),
            TextColor(Color::srgb(1.0, 0.88, 0.56)),
        )],
    )
}

fn battle_action_row(buttons: [(&str, NavigateTo); 4]) -> impl Bundle + use<> {
    (
        Node {
            width: percent(100),
            max_width: px(680),
            flex_direction: FlexDirection::Row,
            column_gap: px(8),
            justify_content: JustifyContent::Center,
            ..default()
        },
        children![
            compact_menu_button(buttons[0].0, buttons[0].1),
            compact_menu_button(buttons[1].0, buttons[1].1),
            compact_menu_button(buttons[2].0, buttons[2].1),
            compact_menu_button(buttons[3].0, buttons[3].1),
        ],
    )
}

fn battle_primary_action(view: &GameViewModel) -> (String, NavigateTo) {
    let Some(summary) = &view.summary else {
        return ("Unavailable".into(), NavigateTo::Noop);
    };
    if summary.pets.len() < 2 {
        return ("Adopt companion".into(), NavigateTo::AddCompanion);
    }
    if summary.active_battles.is_empty() {
        ("Start battle".into(), NavigateTo::StartSelectedBattle)
    } else {
        ("Choose tactic".into(), NavigateTo::Noop)
    }
}

fn breeding_page(view: &GameViewModel, selection: PetSelection) -> impl Bundle + use<> {
    let (primary_label, primary_action, _) = breeding_primary_action(view, selection);
    (
        screen_node(),
        BackgroundColor(Color::srgb(0.055, 0.07, 0.10)),
        children![
            heading("Breeding & eggs"),
            (
                Text::new(breeding_text(view, selection)),
                content_node(),
                TextLayout::justify(Justify::Center),
                TextFont::from_font_size(19.0),
                TextColor(Color::srgb(0.78, 0.82, 0.88)),
            ),
            status_banner(view),
            menu_button("Next parent A", NavigateTo::CyclePrimary),
            menu_button("Next parent B", NavigateTo::CycleSecondary),
            (
                menu_button(&primary_label, primary_action),
                BreedingActionButton,
            ),
            menu_button("Home", NavigateTo::Continue),
        ],
    )
}

fn breeding_primary_action(
    view: &GameViewModel,
    selection: PetSelection,
) -> (String, NavigateTo, bool) {
    let Some(summary) = &view.summary else {
        return ("Unavailable".into(), NavigateTo::Noop, true);
    };
    if summary.pets.len() < 2 {
        return ("Adopt companion".into(), NavigateTo::AddCompanion, false);
    }
    if let Some(egg) = summary.eggs.first() {
        if egg.ready {
            return ("Hatch egg".into(), NavigateTo::HatchFirstEgg, false);
        }
        return (
            format!("Incubating ({} min)", egg.remaining_ticks),
            NavigateTo::Noop,
            true,
        );
    }
    let Some([first, second]) = selection.pair(summary.pets.len()) else {
        return ("Choose two parents".into(), NavigateTo::Noop, true);
    };
    let parents = [&summary.pets[first], &summary.pets[second]];
    if parents[0].id == parents[1].id {
        return (
            "Choose two different parents".into(),
            NavigateTo::Noop,
            true,
        );
    }
    if parents.iter().any(|pet| !pet.alive) {
        return ("Parent is not alive".into(), NavigateTo::Noop, true);
    }
    if parents
        .iter()
        .any(|pet| pet.life_stage != "adult" || pet.age_ticks < 7 * 1_440)
    {
        return ("Parents need 7 days".into(), NavigateTo::Noop, true);
    }
    (
        "Breed selected pair".into(),
        NavigateTo::BreedSelectedPair,
        false,
    )
}

pub(crate) fn spawn_inventory(mut commands: Commands<'_, '_>, view: Res<'_, GameViewModel>) {
    commands.spawn((
        ScreenRoot,
        screen_node(),
        BackgroundColor(Color::srgb(0.055, 0.07, 0.10)),
        children![
            heading("Inventory"),
            (
                Text::new(inventory_text(&view)),
                InventoryDetails,
                content_node(),
                TextLayout::justify(Justify::Center),
                TextFont::from_font_size(19.0),
                TextColor(Color::srgb(0.78, 0.82, 0.88)),
            ),
            status_banner(&view),
            menu_button("Pet status", NavigateTo::PetStatus),
            menu_button("Home", NavigateTo::Continue),
        ],
    ));
}

fn heading(label: &str) -> impl Bundle + use<> {
    (
        Text::new(label.to_owned()),
        content_node(),
        TextLayout::justify(Justify::Center),
        TextFont::from_font_size(42.0),
        TextColor(Color::srgb(0.95, 0.78, 0.38)),
    )
}

fn battle_screen_node() -> Node {
    Node {
        width: percent(100),
        height: percent(100),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        row_gap: px(9),
        padding: UiRect::all(px(12)),
        overflow: Overflow::clip(),
        ..default()
    }
}

fn screen_node() -> Node {
    Node {
        width: percent(100),
        height: percent(100),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        row_gap: px(18),
        padding: UiRect::all(px(24)),
        overflow: Overflow::clip(),
        ..default()
    }
}

fn content_node() -> Node {
    Node {
        width: percent(100),
        max_width: px(720),
        ..default()
    }
}

fn status_banner(view: &GameViewModel) -> impl Bundle + use<> {
    let (text, is_error) = status_message(view);
    let mut accessibility =
        AccessibilityTreeNode::new(if is_error { Role::Alert } else { Role::Status });
    accessibility.set_label(text.clone());
    (
        Text::new(text.clone()),
        StatusDetails,
        Node {
            width: percent(100),
            max_width: px(620),
            min_height: px(36),
            padding: UiRect::axes(px(12), px(7)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(10)),
            ..default()
        },
        BackgroundColor(status_background(is_error)),
        BorderColor::all(status_border(is_error)),
        TextLayout::justify(Justify::Center),
        TextFont::from_font_size(17.0),
        TextColor(status_color(is_error)),
        if text.is_empty() {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        },
        AccessibilityNode(accessibility),
    )
}

fn menu_button(label: &str, action: NavigateTo) -> impl Bundle + use<> {
    let mut accessibility = AccessibilityTreeNode::new(Role::Button);
    accessibility.set_label(label);
    (
        bevy::ui_widgets::Button,
        action,
        TabIndex(0),
        Hovered::default(),
        AccessibilityNode(accessibility),
        Node {
            width: percent(100),
            max_width: px(280),
            min_height: px(54),
            border: UiRect::all(px(3)),
            padding: UiRect::axes(px(12), px(8)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border_radius: BorderRadius::all(px(12)),
            ..default()
        },
        BorderColor::all(Color::NONE),
        BackgroundColor(Color::srgb(0.16, 0.25, 0.32)),
        children![(
            Text::new(label.to_owned()),
            TextFont::from_font_size(22.0),
            TextColor(Color::WHITE),
        )],
    )
}

fn compact_menu_button(label: &str, action: NavigateTo) -> impl Bundle + use<> {
    let mut accessibility = AccessibilityTreeNode::new(Role::Button);
    accessibility.set_label(label);
    (
        bevy::ui_widgets::Button,
        action,
        TabIndex(0),
        Hovered::default(),
        AccessibilityNode(accessibility),
        Node {
            flex_grow: 1.0,
            flex_basis: px(0),
            min_height: px(46),
            border: UiRect::all(px(3)),
            padding: UiRect::axes(px(6), px(6)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border_radius: BorderRadius::all(px(10)),
            ..default()
        },
        BorderColor::all(Color::NONE),
        BackgroundColor(Color::srgb(0.16, 0.25, 0.32)),
        children![(
            Text::new(label.to_owned()),
            TextFont::from_font_size(16.0),
            TextColor(Color::WHITE),
        )],
    )
}

pub(crate) fn activate_menu_button(
    activate: On<'_, '_, Activate>,
    actions: Query<'_, '_, &NavigateTo>,
    mut activations: MessageWriter<'_, MenuActivated>,
) {
    if let Ok(action) = actions.get(activate.entity) {
        activations.write(MenuActivated(*action));
    }
}

pub(crate) fn navigate_buttons(
    mut activations: MessageReader<'_, '_, MenuActivated>,
    mut next: ResMut<'_, NextState<AppState>>,
    state: Res<'_, State<AppState>>,
    active: Res<'_, ActiveSaveSlot>,
    mut view: ResMut<'_, GameViewModel>,
    mut selection: ResMut<'_, PetSelection>,
    mut io: NavigationIo<'_, '_>,
) {
    for activation in activations.read() {
        let destination = &activation.0;
        if destination.changes_screen() {
            view.last_notice = None;
        }
        match destination {
            NavigateTo::Continue => {
                if view
                    .summary
                    .as_ref()
                    .is_some_and(|summary| summary.pets.is_empty())
                {
                    next.set(AppState::PetCreation);
                } else {
                    next.set(AppState::Home);
                }
            }
            NavigateTo::MainMenu => next.set(AppState::MainMenu),
            NavigateTo::Save => {
                io.commands
                    .write(AppCommand(Command::Save { slot: active.0 }));
            }
            NavigateTo::RequestReset => next.set(AppState::ResetConfirmation),
            NavigateTo::ConfirmReset => {
                io.commands
                    .write(AppCommand(Command::ResetGame { slot: active.0 }));
            }
            NavigateTo::UnlockAdoptionSlot => {
                io.commands.write(AppCommand(Command::UnlockAdoptionSlot));
            }
            NavigateTo::AdoptEmberling | NavigateTo::AdoptStormling | NavigateTo::AdoptAquafin => {
                let raw_name = io
                    .name_inputs
                    .iter()
                    .next()
                    .map_or_else(|| "Companion".into(), |input| input.value().to_string());
                let name = match PetName::new(raw_name) {
                    Ok(name) => name,
                    Err(error) => {
                        io.failures.write(CommandFailed::user(error.to_string()));
                        continue;
                    }
                };
                let species = match destination {
                    NavigateTo::AdoptEmberling => "emberling",
                    NavigateTo::AdoptStormling => "stormling",
                    NavigateTo::AdoptAquafin => "aquafin",
                    _ => unreachable!("matched adoption actions above"),
                };
                io.commands.write(AppCommand(Command::CreatePet {
                    species: SpeciesId::new(species).expect("built-in species id is valid"),
                    name,
                }));
            }
            NavigateTo::PetStatus => next.set(AppState::PetStatus),
            NavigateTo::Inventory => next.set(AppState::Inventory),
            NavigateTo::FeedSelectedPet => {
                let Some(summary) = &view.summary else {
                    continue;
                };
                let Some(pet) = selection
                    .primary(summary.pets.len())
                    .and_then(|index| summary.pets.get(index))
                else {
                    continue;
                };
                let Ok(pet_id) = pet.id.parse::<PetId>() else {
                    tracing::error!(pet_id = %pet.id, "view model contained an invalid pet id");
                    continue;
                };
                io.commands.write(AppCommand(Command::FeedPet {
                    pet: pet_id,
                    item: ItemId::new("apple_slice").expect("built-in item id is valid"),
                }));
            }
            NavigateTo::CyclePrimary => {
                let pet_count = selectable_pet_count(&view, state.get());
                selection.cycle_primary(pet_count);
            }
            NavigateTo::CycleSecondary => {
                let pet_count = selectable_pet_count(&view, state.get());
                selection.cycle_secondary(pet_count);
            }
            NavigateTo::RequestRelease => next.set(AppState::PetRelease),
            NavigateTo::ConfirmRelease => {
                let Some(summary) = &view.summary else {
                    continue;
                };
                let Some(pet) = selection
                    .primary(summary.pets.len())
                    .and_then(|index| summary.pets.get(index))
                else {
                    continue;
                };
                let Ok(pet) = pet.id.parse::<PetId>() else {
                    tracing::error!(pet_id = %pet.id, "view model contained an invalid pet id");
                    continue;
                };
                io.commands.write(AppCommand(Command::ReleasePet { pet }));
            }
            NavigateTo::Breeding => next.set(AppState::Breeding),
            NavigateTo::AddCompanion => next.set(AppState::PetCreation),
            NavigateTo::BreedSelectedPair => {
                let Some(summary) = &view.summary else {
                    continue;
                };
                let Some([first_index, second_index]) = selection.pair(summary.pets.len()) else {
                    continue;
                };
                let (first, second) = (&summary.pets[first_index], &summary.pets[second_index]);
                let (Ok(first), Ok(second)) =
                    (first.id.parse::<PetId>(), second.id.parse::<PetId>())
                else {
                    tracing::error!("view model contained an invalid breeding pair");
                    continue;
                };
                io.commands
                    .write(AppCommand(Command::Breed { first, second }));
            }
            NavigateTo::HatchFirstEgg => {
                let Some(egg) = view
                    .summary
                    .as_ref()
                    .and_then(|summary| summary.eggs.first())
                else {
                    continue;
                };
                let Ok(egg) = egg.id.parse::<EggId>() else {
                    tracing::error!(egg_id = %egg.id, "view model contained an invalid egg id");
                    continue;
                };
                io.commands.write(AppCommand(Command::HatchEgg { egg }));
            }
            NavigateTo::Noop => {}
            NavigateTo::Battle => next.set(AppState::Battle),
            NavigateTo::StartSelectedBattle => {
                let Some(setup) = &view.battle_setup else {
                    continue;
                };
                let Some([first_index, second_index]) = selection.pair(setup.pets.len()) else {
                    continue;
                };
                let (first, second) = (&setup.pets[first_index], &setup.pets[second_index]);
                let (Ok(first), Ok(second)) =
                    (first.id.parse::<PetId>(), second.id.parse::<PetId>())
                else {
                    tracing::error!("view model contained an invalid battle pair");
                    continue;
                };
                io.commands
                    .write(AppCommand(Command::StartBattle { first, second }));
            }
            NavigateTo::ResolveBattlePower
            | NavigateTo::ResolveBattlePrecision
            | NavigateTo::ResolveBattleEfficient => {
                let tactic = match destination {
                    NavigateTo::ResolveBattlePower => BattleTactic::Power,
                    NavigateTo::ResolveBattlePrecision => BattleTactic::Precision,
                    _ => BattleTactic::Efficient,
                };
                let Some((battle, actions)) = battle_actions(&view, tactic) else {
                    continue;
                };
                io.commands
                    .write(AppCommand(Command::ResolveBattle { battle, actions }));
            }
            NavigateTo::SaveSlots => next.set(AppState::SaveSlots),
            NavigateTo::LoadNextSlot => {
                if let Some(next_slot) = next_save_slot(&view, active.0) {
                    io.switches.write(SwitchSaveSlot(next_slot));
                }
            }
            NavigateTo::ToggleAudio | NavigateTo::ToggleReducedMotion => {}
        }
    }
}

pub(crate) fn handle_preference_buttons(
    mut activations: MessageReader<'_, '_, MenuActivated>,
    mut preferences: ResMut<'_, PresentationPreferences>,
    mut view: ResMut<'_, GameViewModel>,
) {
    for activation in activations.read() {
        let notice = match activation.0 {
            NavigateTo::ToggleAudio => {
                preferences.audio_enabled = !preferences.audio_enabled;
                if preferences.audio_enabled {
                    Some("Sound effects enabled.")
                } else {
                    Some("Sound effects muted.")
                }
            }
            NavigateTo::ToggleReducedMotion => {
                preferences.reduced_motion = !preferences.reduced_motion;
                if preferences.reduced_motion {
                    Some("Reduced motion enabled.")
                } else {
                    Some("Full motion enabled.")
                }
            }
            _ => None,
        };
        if let Some(notice) = notice {
            view.last_error = None;
            view.last_notice = Some(notice.into());
        }
    }
}

pub(crate) fn refresh_preference_text(
    preferences: Res<'_, PresentationPreferences>,
    mut text: Query<'_, '_, &mut Text, With<PreferenceDetails>>,
) {
    if !preferences.is_changed() {
        return;
    }
    for mut value in &mut text {
        **value = preference_text(*preferences);
    }
}

pub(crate) fn refresh_preference_buttons(
    preferences: Res<'_, PresentationPreferences>,
    mut audio: Query<
        '_,
        '_,
        (&Children, &mut AccessibilityNode),
        (With<AudioPreferenceButton>, Without<MotionPreferenceButton>),
    >,
    mut motion: Query<
        '_,
        '_,
        (&Children, &mut AccessibilityNode),
        (With<MotionPreferenceButton>, Without<AudioPreferenceButton>),
    >,
    mut labels: Query<'_, '_, &mut Text>,
) {
    if !preferences.is_changed() {
        return;
    }
    for (children, mut accessibility) in &mut audio {
        update_button_label(
            children,
            &mut accessibility,
            preference_button_label(*preferences, true),
            &mut labels,
        );
    }
    for (children, mut accessibility) in &mut motion {
        update_button_label(
            children,
            &mut accessibility,
            preference_button_label(*preferences, false),
            &mut labels,
        );
    }
}

fn update_button_label(
    children: &Children,
    accessibility: &mut AccessibilityNode,
    label: &str,
    labels: &mut Query<'_, '_, &mut Text>,
) {
    accessibility.set_label(label);
    for child in children.iter() {
        if let Ok(mut text) = labels.get_mut(child) {
            **text = label.into();
        }
    }
}

fn preference_button_label(preferences: PresentationPreferences, audio: bool) -> &'static str {
    if audio {
        if preferences.audio_enabled {
            "Sound: On"
        } else {
            "Sound: Off"
        }
    } else if preferences.reduced_motion {
        "Motion: Reduced"
    } else {
        "Motion: Full"
    }
}

fn preference_text(preferences: PresentationPreferences) -> String {
    format!(
        "Sound: {}  |  Motion: {}",
        if preferences.audio_enabled {
            "on"
        } else {
            "muted"
        },
        if preferences.reduced_motion {
            "reduced"
        } else {
            "full"
        }
    )
}

fn selectable_pet_count(view: &GameViewModel, state: &AppState) -> usize {
    if *state == AppState::Battle {
        view.battle_setup
            .as_ref()
            .map_or(0, |setup| setup.pets.len())
    } else {
        view.summary
            .as_ref()
            .map_or(0, |summary| summary.pets.len())
    }
}

fn next_save_slot(
    view: &GameViewModel,
    active: domain::identifiers::SaveSlotId,
) -> Option<domain::identifiers::SaveSlotId> {
    if view.save_slots.len() < 2 {
        return None;
    }
    let active_text = active.to_string();
    let current = view
        .save_slots
        .iter()
        .position(|slot| slot.id == active_text)
        .unwrap_or(0);
    view.save_slots[(current + 1) % view.save_slots.len()]
        .id
        .parse()
        .map_err(|error| tracing::error!(%error, "view model contained an invalid save slot id"))
        .ok()
}

pub(crate) fn refresh_battle_screen(
    mut commands: Commands<'_, '_>,
    asset_server: Res<'_, AssetServer>,
    state: Res<'_, State<AppState>>,
    view: Res<'_, GameViewModel>,
    selection: Res<'_, PetSelection>,
    roots: Query<'_, '_, Entity, With<ScreenRoot>>,
) {
    if *state.get() != AppState::Battle || (!view.is_changed() && !selection.is_changed()) {
        return;
    }
    for entity in &roots {
        commands.entity(entity).despawn();
    }
    commands.spawn((ScreenRoot, battle_page(&asset_server, &view, *selection)));
}

pub(crate) fn refresh_breeding_screen(
    mut commands: Commands<'_, '_>,
    state: Res<'_, State<AppState>>,
    view: Res<'_, GameViewModel>,
    selection: Res<'_, PetSelection>,
    roots: Query<'_, '_, Entity, With<ScreenRoot>>,
) {
    if *state.get() != AppState::Breeding || (!view.is_changed() && !selection.is_changed()) {
        return;
    }
    for entity in &roots {
        commands.entity(entity).despawn();
    }
    commands.spawn((ScreenRoot, breeding_page(&view, *selection)));
}

pub(crate) fn refresh_pet_status_text(
    view: Res<'_, GameViewModel>,
    selection: Res<'_, PetSelection>,
    mut text: Query<'_, '_, &mut Text, With<PetStatusDetails>>,
) {
    if !view.is_changed() && !selection.is_changed() {
        return;
    }
    for mut value in &mut text {
        **value = pet_status_text(&view, *selection);
    }
}

pub(crate) fn refresh_pet_status_portrait(
    asset_server: Res<'_, AssetServer>,
    view: Res<'_, GameViewModel>,
    selection: Res<'_, PetSelection>,
    mut portraits: Query<'_, '_, (&mut ImageNode, &mut AccessibilityNode), With<PetStatusPortrait>>,
) {
    if !view.is_changed() && !selection.is_changed() {
        return;
    }
    let Some(pet) = selected_pet(&view, *selection) else {
        return;
    };
    for (mut image, mut accessibility) in &mut portraits {
        image.image = asset_server.load(sprite_path(&pet.species));
        accessibility.set_label(format!(
            "{}, {} portrait",
            pet.name,
            display_name(&pet.species)
        ));
    }
}

pub(crate) fn refresh_need_meters(
    view: Res<'_, GameViewModel>,
    selection: Res<'_, PetSelection>,
    mut fills: Query<'_, '_, (&PetNeedFill, &mut Node, &mut BackgroundColor)>,
    mut values: Query<'_, '_, (&PetNeedValue, &mut Text)>,
) {
    if !view.is_changed() && !selection.is_changed() {
        return;
    }
    let Some(pet) = selected_pet(&view, *selection) else {
        return;
    };
    for (marker, mut node, mut color) in &mut fills {
        if let Some(need) = pet.needs.get(marker.0) {
            node.width = percent((need.value_milli / 1_000).clamp(0, 100) as f32);
            color.0 = need_color(need.value_milli);
        }
    }
    for (marker, mut text) in &mut values {
        if let Some(need) = pet.needs.get(marker.0) {
            **text = need_value_text(&need.kind, need.value_milli);
        }
    }
}

pub(crate) fn refresh_care_action(
    mut commands: Commands<'_, '_>,
    view: Res<'_, GameViewModel>,
    selection: Res<'_, PetSelection>,
    mut buttons: Query<
        '_,
        '_,
        (
            Entity,
            &mut NavigateTo,
            &mut AccessibilityNode,
            Has<InteractionDisabled>,
        ),
        With<CareActionButton>,
    >,
    mut titles: Query<
        '_,
        '_,
        (&mut Text, &mut TextColor),
        (With<CareActionTitle>, Without<CareActionDetail>),
    >,
    mut details: Query<
        '_,
        '_,
        (&mut Text, &mut TextColor),
        (With<CareActionDetail>, Without<CareActionTitle>),
    >,
) {
    let (title, detail, action, disabled) = care_action(&view, *selection);
    for (entity, mut current_action, mut accessibility, was_disabled) in &mut buttons {
        *current_action = action;
        accessibility.set_label(format!("{title}, {detail}"));
        for (mut text, mut color) in &mut titles {
            **text = title.clone();
            color.0 = if disabled {
                Color::srgb(0.52, 0.57, 0.62)
            } else {
                Color::WHITE
            };
        }
        for (mut text, mut color) in &mut details {
            **text = detail.clone();
            color.0 = if disabled {
                Color::srgb(0.43, 0.48, 0.53)
            } else {
                Color::srgb(0.70, 0.78, 0.84)
            };
        }
        if disabled && !was_disabled {
            commands.entity(entity).insert(InteractionDisabled);
        } else if !disabled && was_disabled {
            commands.entity(entity).remove::<InteractionDisabled>();
        }
    }
}

pub(crate) fn refresh_breeding_action(
    mut commands: Commands<'_, '_>,
    view: Res<'_, GameViewModel>,
    selection: Res<'_, PetSelection>,
    mut buttons: Query<
        '_,
        '_,
        (
            Entity,
            &mut NavigateTo,
            &Children,
            &mut AccessibilityNode,
            Has<InteractionDisabled>,
        ),
        With<BreedingActionButton>,
    >,
    mut labels: Query<'_, '_, (&mut Text, &mut TextColor)>,
) {
    let (label, action, disabled) = breeding_primary_action(&view, *selection);
    for (entity, mut current_action, children, mut accessibility, was_disabled) in &mut buttons {
        *current_action = action;
        accessibility.set_label(label.clone());
        for child in children.iter() {
            if let Ok((mut text, mut color)) = labels.get_mut(child) {
                **text = label.clone();
                color.0 = if disabled {
                    Color::srgb(0.52, 0.57, 0.62)
                } else {
                    Color::WHITE
                };
            }
        }
        if disabled && !was_disabled {
            commands.entity(entity).insert(InteractionDisabled);
        } else if !disabled && was_disabled {
            commands.entity(entity).remove::<InteractionDisabled>();
        }
    }
}

pub(crate) fn refresh_pet_cycle_action(
    mut commands: Commands<'_, '_>,
    view: Res<'_, GameViewModel>,
    mut buttons: Query<
        '_,
        '_,
        (
            Entity,
            &mut NavigateTo,
            &Children,
            &mut AccessibilityNode,
            Has<InteractionDisabled>,
        ),
        With<PetCycleButton>,
    >,
    mut labels: Query<'_, '_, (&mut Text, &mut TextColor)>,
) {
    let (label, action, disabled) = pet_cycle_action(&view);
    for (entity, mut current_action, children, mut accessibility, was_disabled) in &mut buttons {
        *current_action = action;
        accessibility.set_label(label.clone());
        for child in children.iter() {
            if let Ok((mut text, mut color)) = labels.get_mut(child) {
                **text = label.clone();
                color.0 = if disabled {
                    Color::srgb(0.52, 0.57, 0.62)
                } else {
                    Color::WHITE
                };
            }
        }
        if disabled && !was_disabled {
            commands.entity(entity).insert(InteractionDisabled);
        } else if !disabled && was_disabled {
            commands.entity(entity).remove::<InteractionDisabled>();
        }
    }
}

pub(crate) fn refresh_inventory_text(
    view: Res<'_, GameViewModel>,
    mut text: Query<'_, '_, &mut Text, With<InventoryDetails>>,
) {
    if !view.is_changed() {
        return;
    }
    for mut value in &mut text {
        **value = inventory_text(&view);
    }
}

pub(crate) fn refresh_save_slot_text(
    view: Res<'_, GameViewModel>,
    active: Res<'_, ActiveSaveSlot>,
    mut text: Query<'_, '_, &mut Text, With<SaveSlotDetails>>,
) {
    if !view.is_changed() && !active.is_changed() {
        return;
    }
    for mut value in &mut text {
        **value = save_slots_text(&view, active.0);
    }
}

pub(crate) fn refresh_home_text(
    view: Res<'_, GameViewModel>,
    mut text: Query<'_, '_, &mut Text, With<HomeDetails>>,
) {
    if !view.is_changed() {
        return;
    }
    for mut value in &mut text {
        **value = home_text(&view);
    }
}

pub(crate) fn refresh_home_pets(
    view: Res<'_, GameViewModel>,
    asset_server: Res<'_, AssetServer>,
    mut portraits: Query<'_, '_, (&PetPortrait, &mut ImageNode, &mut AccessibilityNode)>,
    mut names: Query<'_, '_, (&PetNameLabel, &mut Text), Without<PetSpeciesLabel>>,
    mut species: Query<'_, '_, (&PetSpeciesLabel, &mut Text), Without<PetNameLabel>>,
) {
    if !view.is_changed() {
        return;
    }
    let Some(summary) = &view.summary else {
        return;
    };
    for (marker, mut image, mut accessibility) in &mut portraits {
        if let Some(pet) = summary.pets.get(marker.0) {
            image.image = asset_server.load(sprite_path(&pet.species));
            accessibility.set_label(format!(
                "{}, {}, pet {}",
                pet.name,
                display_name(&pet.species),
                marker.0 + 1
            ));
        }
    }
    for (marker, mut text) in &mut names {
        if let Some(pet) = summary.pets.get(marker.0) {
            **text = pet.name.clone();
        }
    }
    for (marker, mut text) in &mut species {
        if let Some(pet) = summary.pets.get(marker.0) {
            **text = display_name(&pet.species);
        }
    }
}

pub(crate) fn refresh_status_text(
    view: Res<'_, GameViewModel>,
    mut status: Query<
        '_,
        '_,
        (
            &mut Text,
            &mut TextColor,
            &mut Visibility,
            &mut AccessibilityNode,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        With<StatusDetails>,
    >,
) {
    if !view.is_changed() {
        return;
    }
    let (message, is_error) = status_message(&view);
    for (mut value, mut color, mut visibility, mut accessibility, mut background, mut border) in
        &mut status
    {
        **value = message.clone();
        color.0 = status_color(is_error);
        *visibility = if message.is_empty() {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        accessibility.set_role(if is_error { Role::Alert } else { Role::Status });
        accessibility.set_label(message.clone());
        background.0 = status_background(is_error);
        *border = BorderColor::all(status_border(is_error));
    }
}

pub(crate) fn style_buttons(
    focus: Res<'_, InputFocus>,
    focus_visible: Res<'_, InputFocusVisible>,
    mut buttons: Query<
        '_,
        '_,
        (
            Entity,
            &Hovered,
            Has<Pressed>,
            Has<InteractionDisabled>,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        With<bevy::ui_widgets::Button>,
    >,
) {
    for (entity, hovered, pressed, disabled, mut background, mut border) in &mut buttons {
        let focused = focus_visible.0 && focus.get() == Some(entity);
        let (fill, outline) = button_style(pressed, hovered.get(), focused, disabled);
        background.0 = fill;
        border.set_all(outline);
    }
}

fn button_style(pressed: bool, hovered: bool, focused: bool, disabled: bool) -> (Color, Color) {
    let fill = match (pressed, hovered, focused, disabled) {
        (_, _, _, true) => Color::srgb(0.10, 0.13, 0.16),
        (true, _, _, false) => Color::srgb(0.20, 0.48, 0.38),
        (false, true, _, false) => Color::srgb(0.22, 0.34, 0.43),
        (false, false, true, false) => Color::srgb(0.18, 0.31, 0.42),
        (false, false, false, false) => Color::srgb(0.16, 0.25, 0.32),
    };
    let outline = if focused {
        Color::srgb(0.95, 0.78, 0.38)
    } else {
        Color::NONE
    };
    (fill, outline)
}

fn home_text(view: &GameViewModel) -> String {
    view.summary.as_ref().map_or_else(
        || "No world loaded".to_owned(),
        |summary| {
            format!(
                "Day {}  |  {} pets  |  Adopted {}/{}  |  {} eggs  |  {} coins",
                summary.tick / 1_440,
                summary.pets.len(),
                summary.adopted_pets,
                summary.max_adopted_pets,
                summary.eggs.len(),
                summary.balance
            )
        },
    )
}

fn status_message(view: &GameViewModel) -> (String, bool) {
    if let Some(error) = &view.last_error {
        (format!("Error: {error}"), true)
    } else if let Some(notice) = &view.last_notice {
        (notice.clone(), false)
    } else {
        (String::new(), false)
    }
}

fn status_color(is_error: bool) -> Color {
    if is_error {
        Color::srgb(1.0, 0.48, 0.42)
    } else {
        Color::srgb(0.48, 0.90, 0.66)
    }
}

fn status_background(is_error: bool) -> Color {
    if is_error {
        Color::srgb(0.19, 0.075, 0.07)
    } else {
        Color::srgb(0.065, 0.16, 0.12)
    }
}

fn status_border(is_error: bool) -> Color {
    if is_error {
        Color::srgb(0.64, 0.22, 0.18)
    } else {
        Color::srgb(0.20, 0.50, 0.34)
    }
}

fn pet_status_text(view: &GameViewModel, selection: PetSelection) -> String {
    let Some(summary) = &view.summary else {
        return "No world loaded".to_owned();
    };
    let Some(index) = selection.primary(summary.pets.len()) else {
        return "No pet has been adopted".to_owned();
    };
    let pet = &summary.pets[index];
    format!(
        "{} the {} | {} | Mood: {}\nLevel {} | XP {}/{} ({}%)\nActivity: {} | Age: {} days | Pet {} of {}",
        pet.name,
        display_name(&pet.species),
        display_name(&pet.life_stage),
        display_name(&pet.mood),
        pet.level,
        pet.experience,
        pet.experience_for_next_level,
        pet.experience_progress_milli / 10,
        display_name(&pet.activity),
        pet.age_ticks / 1_440,
        index + 1,
        summary.pets.len()
    )
}

fn selected_pet(
    view: &GameViewModel,
    selection: PetSelection,
) -> Option<&application::dto::PetDto> {
    let summary = view.summary.as_ref()?;
    selection
        .primary(summary.pets.len())
        .and_then(|index| summary.pets.get(index))
}

fn apple_count(view: &GameViewModel) -> u32 {
    view.summary.as_ref().map_or(0, |summary| {
        summary
            .inventory
            .iter()
            .find(|item| item.item_id == "apple_slice")
            .map_or(0, |item| item.quantity)
    })
}

fn care_action(
    view: &GameViewModel,
    selection: PetSelection,
) -> (String, String, NavigateTo, bool) {
    let Some(pet) = selected_pet(view, selection) else {
        return (
            "Care unavailable".into(),
            "No pet selected".into(),
            NavigateTo::Noop,
            true,
        );
    };
    if !pet.alive {
        return (
            "Care unavailable".into(),
            "This pet cannot be fed".into(),
            NavigateTo::Noop,
            true,
        );
    }
    let hunger = pet
        .needs
        .iter()
        .find(|need| need.kind == "hunger")
        .map_or(100_000, |need| need.value_milli);
    if hunger >= 100_000 {
        return (
            "Pet is full".into(),
            "No food needed".into(),
            NavigateTo::Noop,
            true,
        );
    }
    let apples = apple_count(view);
    if apples == 0 {
        return (
            "No apple slices".into(),
            "Inventory is empty".into(),
            NavigateTo::Noop,
            true,
        );
    }
    (
        "Feed apple".into(),
        format!("{apples} slices available"),
        NavigateTo::FeedSelectedPet,
        false,
    )
}

fn need_value_text(kind: &str, value_milli: i32) -> String {
    let percent_value = (value_milli / 1_000).clamp(0, 100);
    let label = if kind == "hunger" {
        "Fullness".to_owned()
    } else {
        display_name(kind)
    };
    format!("{} {}% - {}", label, percent_value, need_state(value_milli))
}

fn need_state(value_milli: i32) -> &'static str {
    match value_milli {
        80_000.. => "Full",
        40_000.. => "Good",
        11_000.. => "Low",
        _ => "Critical",
    }
}

fn need_color(value_milli: i32) -> Color {
    match value_milli {
        80_000.. => Color::srgb(0.28, 0.78, 0.52),
        40_000.. => Color::srgb(0.92, 0.68, 0.24),
        11_000.. => Color::srgb(0.95, 0.43, 0.22),
        _ => Color::srgb(0.92, 0.24, 0.24),
    }
}

fn display_name(raw: &str) -> String {
    let mut value = raw.replace('_', " ");
    if let Some(first) = value.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    value
}

fn inventory_text(view: &GameViewModel) -> String {
    let Some(summary) = &view.summary else {
        return "No world loaded".to_owned();
    };
    if summary.inventory.is_empty() {
        return "Your inventory is empty".to_owned();
    }
    summary
        .inventory
        .iter()
        .map(|item| format!("{}  ×{}", item.item_id, item.quantity))
        .collect::<Vec<_>>()
        .join("\n")
}

fn save_slots_text(view: &GameViewModel, active: domain::identifiers::SaveSlotId) -> String {
    if view.save_slots.is_empty() {
        return format!("Active: {active}\nNot saved yet");
    }
    view.save_slots
        .iter()
        .map(|slot| {
            let marker = if slot.id == active.to_string() {
                "active"
            } else {
                "available"
            };
            format!(
                "{} | schema {} | updated {} | {marker}",
                slot.id, slot.schema_version, slot.updated_at_unix
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn breeding_text(view: &GameViewModel, selection: PetSelection) -> String {
    let Some(summary) = &view.summary else {
        return "No world loaded".to_owned();
    };
    let mut lines = selection.pair(summary.pets.len()).map_or_else(
        || {
            summary
                .pets
                .first()
                .into_iter()
                .map(|pet| {
                    format!(
                        "Parent A: {} ({}) | {} | age {} days",
                        pet.name,
                        display_name(&pet.species),
                        pet.life_stage,
                        pet.age_ticks / 1_440
                    )
                })
                .collect::<Vec<_>>()
        },
        |[first, second]| {
            [(&summary.pets[first], 'A'), (&summary.pets[second], 'B')]
                .into_iter()
                .map(|(pet, label)| {
                    format!(
                        "Parent {label}: {} ({}) | {} | age {} days",
                        pet.name,
                        display_name(&pet.species),
                        pet.life_stage,
                        pet.age_ticks / 1_440
                    )
                })
                .collect::<Vec<_>>()
        },
    );
    if summary.pets.len() < 2 {
        lines.push("Two founder pets are required.".into());
    } else if let Some(egg) = summary.eggs.first() {
        let readiness = if egg.ready {
            "ready to hatch".to_owned()
        } else {
            format!("{} minutes remaining", egg.remaining_ticks)
        };
        lines.push(format!("Egg: {} | {readiness}", display_name(&egg.species)));
    } else {
        lines.push("Both parents must be healthy adults at least 7 days old.".into());
    }
    lines.join("\n")
}

fn battle_actions(
    view: &GameViewModel,
    tactic: BattleTactic,
) -> Option<(BattleId, Vec<[BattleAction; 2]>)> {
    let summary = view.summary.as_ref()?;
    let battle = summary.active_battles.first()?;
    let setup = view.battle_setup.as_ref()?;
    let first = setup
        .pets
        .iter()
        .find(|pet| pet.id == battle.combatant_ids[0])?;
    let second = setup
        .pets
        .iter()
        .find(|pet| pet.id == battle.combatant_ids[1])?;
    let first_move = tactic_move(first, tactic)?;
    let second_move = tactic_move(second, tactic)?;
    let turns = usize::from(
        (100 / first_move.energy_cost.max(1)).min(100 / second_move.energy_cost.max(1)),
    )
    .max(1);
    let actors = [
        battle.combatant_ids[0].parse::<PetId>().ok()?,
        battle.combatant_ids[1].parse::<PetId>().ok()?,
    ];
    let moves = [
        MoveId::new(&first_move.id).ok()?,
        MoveId::new(&second_move.id).ok()?,
    ];
    let actions = (0..turns)
        .map(|_| {
            [
                BattleAction {
                    actor: actors[0],
                    move_id: moves[0].clone(),
                },
                BattleAction {
                    actor: actors[1],
                    move_id: moves[1].clone(),
                },
            ]
        })
        .collect();
    Some((battle.id.parse::<BattleId>().ok()?, actions))
}

fn tactic_move(
    pet: &application::dto::BattlePetDto,
    tactic: BattleTactic,
) -> Option<&application::dto::BattleMoveDto> {
    match tactic {
        BattleTactic::Power => pet
            .moves
            .iter()
            .max_by_key(|battle_move| (battle_move.power, battle_move.accuracy_milli)),
        BattleTactic::Precision => pet
            .moves
            .iter()
            .max_by_key(|battle_move| (battle_move.accuracy_milli, battle_move.power)),
        BattleTactic::Efficient => pet.moves.iter().max_by_key(|battle_move| {
            (
                u32::from(battle_move.power) * 1_000 / u32::from(battle_move.energy_cost.max(1)),
                battle_move.accuracy_milli,
            )
        }),
    }
}

pub(crate) fn despawn_screen(
    mut commands: Commands<'_, '_>,
    roots: Query<'_, '_, Entity, With<ScreenRoot>>,
) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use application::dto::{
        BattleDto, BattleMoveDto, BattlePetDto, BattleSetupDto, GameSummaryDto, InventoryItemDto,
        NeedDto, PetDto, SaveSlotDto,
    };
    use domain::identifiers::{BattleId, PetId};
    use domain::rng::WorldSeed;
    use domain::time::Tick;

    use super::*;

    fn view() -> GameViewModel {
        GameViewModel {
            summary: Some(GameSummaryDto {
                tick: 0,
                balance: 0,
                clamped_offline_ticks: 0,
                adopted_pets: 1,
                max_adopted_pets: application::BASE_ADOPTED_PETS,
                maximum_adopted_pets: application::MAX_ADOPTED_PETS,
                next_adoption_slot_cost: Some(100),
                pets: vec![PetDto {
                    id: "pet-id".into(),
                    name: "Sparky".into(),
                    species: "emberling".into(),
                    life_stage: "hatchling".into(),
                    mood: "happy".into(),
                    activity: "idle".into(),
                    alive: true,
                    age_ticks: 0,
                    level: 1,
                    experience: 0,
                    experience_for_next_level: 100,
                    experience_progress_milli: 0,
                    needs: vec![NeedDto {
                        kind: "hunger".into(),
                        value_milli: 82_000,
                    }],
                }],
                eggs: Vec::new(),
                active_battles: Vec::new(),
                inventory: vec![InventoryItemDto {
                    item_id: "apple_slice".into(),
                    quantity: 4,
                }],
            }),
            battle_setup: None,
            last_battle_result: None,
            save_slots: Vec::new(),
            last_notice: None,
            last_error: None,
        }
    }

    #[test]
    fn menu_buttons_expose_keyboard_and_screen_reader_metadata() {
        let mut world = World::new();
        let root = world.spawn(ScreenRoot).id();
        let button = world
            .spawn(menu_button("Continue", NavigateTo::Continue))
            .id();

        assert!(world.entity(root).contains::<TabGroup>());
        assert!(world.entity(button).contains::<bevy::ui_widgets::Button>());
        assert_eq!(world.get::<TabIndex>(button), Some(&TabIndex(0)));
        let accessibility = world.get::<AccessibilityNode>(button).unwrap();
        assert_eq!(accessibility.role(), Role::Button);
        assert_eq!(accessibility.label(), Some("Continue"));
    }

    #[test]
    fn widget_activation_enters_the_shared_navigation_message_path() {
        let mut app = App::new();
        app.add_message::<MenuActivated>()
            .add_observer(activate_menu_button);
        let button = app
            .world_mut()
            .spawn(menu_button("Continue", NavigateTo::Continue))
            .id();

        app.world_mut().trigger(Activate { entity: button });

        let messages = app.world().resource::<Messages<MenuActivated>>();
        let mut cursor = messages.get_cursor();
        assert_eq!(
            cursor.read(messages).next().map(|message| message.0),
            Some(NavigateTo::Continue)
        );
    }

    #[test]
    fn keyboard_focus_has_a_visible_high_contrast_outline() {
        let (normal_fill, normal_outline) = button_style(false, false, false, false);
        let (focused_fill, focused_outline) = button_style(false, false, true, false);

        assert_ne!(normal_fill, focused_fill);
        assert_eq!(normal_outline, Color::NONE);
        assert_ne!(focused_outline, Color::NONE);
    }

    #[test]
    fn responsive_shell_and_buttons_fit_narrow_windows() {
        let shell = screen_node();
        assert_eq!(shell.width, percent(100));
        assert_eq!(shell.padding, UiRect::all(px(24)));

        let mut world = World::new();
        let button = world
            .spawn(menu_button("Continue", NavigateTo::Continue))
            .id();
        let node = world.get::<Node>(button).unwrap();
        assert_eq!(node.width, percent(100));
        assert_eq!(node.max_width, px(280));
        assert_eq!(node.min_height, px(54));
    }

    #[test]
    fn errors_take_priority_over_success_notices() {
        let mut view = view();
        view.last_notice = Some("Game saved.".into());
        assert_eq!(status_message(&view), ("Game saved.".into(), false));

        view.last_error = Some("disk full".into());
        assert_eq!(status_message(&view), ("Error: disk full".into(), true));
    }

    #[test]
    fn home_summary_tracks_live_view_changes() {
        let mut view = view();
        assert!(home_text(&view).contains("Day 0"));
        view.summary.as_mut().unwrap().tick = 2_880;
        view.summary.as_mut().unwrap().balance = 25;
        let text = home_text(&view);
        assert!(text.contains("Day 2"));
        assert!(text.contains("25 coins"));
    }

    #[test]
    fn pet_status_contains_care_relevant_information() {
        let view = view();
        let text = pet_status_text(&view, PetSelection::default());
        assert!(text.contains("Emberling"));
        assert!(text.contains("Mood: Happy"));
        assert!(text.contains("Activity: Idle"));
        assert_eq!(
            care_action(&view, PetSelection::default()),
            (
                "Feed apple".into(),
                "4 slices available".into(),
                NavigateTo::FeedSelectedPet,
                false
            )
        );
        assert_eq!(need_value_text("hunger", 82_000), "Fullness 82% - Full");
    }

    #[test]
    fn full_hunger_disables_feeding_before_it_can_fail() {
        let mut view = view();
        view.summary.as_mut().unwrap().pets[0].needs[0].value_milli = 100_000;

        assert_eq!(
            care_action(&view, PetSelection::default()),
            (
                "Pet is full".into(),
                "No food needed".into(),
                NavigateTo::Noop,
                true
            )
        );
    }

    #[test]
    fn pet_cycle_action_explains_when_only_one_pet_exists() {
        assert_eq!(
            pet_cycle_action(&view()),
            ("Only one pet".into(), NavigateTo::Noop, true)
        );
    }

    #[test]
    fn every_authored_species_has_a_distinct_sprite() {
        assert_eq!(sprite_path("emberling"), "sprites/emberling.png");
        assert_eq!(sprite_path("stormling"), "sprites/stormling.png");
        assert_eq!(sprite_path("aquafin"), "sprites/aquafin.png");
    }

    #[test]
    fn preference_buttons_show_state_without_toggle_wording() {
        let mut preferences = PresentationPreferences::default();
        assert_eq!(preference_button_label(preferences, true), "Sound: On");
        assert_eq!(preference_button_label(preferences, false), "Motion: Full");
        preferences.audio_enabled = false;
        preferences.reduced_motion = true;
        assert_eq!(preference_button_label(preferences, true), "Sound: Off");
        assert_eq!(
            preference_button_label(preferences, false),
            "Motion: Reduced"
        );
    }

    #[test]
    fn need_states_are_understandable_without_color() {
        assert_eq!(need_state(100_000), "Full");
        assert_eq!(need_state(50_000), "Good");
        assert_eq!(need_state(20_000), "Low");
        assert_eq!(need_state(5_000), "Critical");
    }

    #[test]
    fn inventory_is_rendered_in_stable_dto_order() {
        assert_eq!(inventory_text(&view()), "apple_slice  ×4");
    }

    #[test]
    fn breeding_primary_action_tracks_onboarding_state() {
        let mut view = view();
        assert_eq!(
            breeding_primary_action(&view, PetSelection::default()),
            ("Adopt companion".into(), NavigateTo::AddCompanion, false)
        );

        let mut companion = view.summary.as_ref().unwrap().pets[0].clone();
        companion.id = "second-pet".into();
        view.summary.as_mut().unwrap().pets.push(companion);
        assert_eq!(
            breeding_primary_action(&view, PetSelection::default()),
            ("Parents need 7 days".into(), NavigateTo::Noop, true)
        );
        for pet in &mut view.summary.as_mut().unwrap().pets {
            pet.life_stage = "adult".into();
            pet.age_ticks = 7 * 1_440;
        }
        assert_eq!(
            breeding_primary_action(&view, PetSelection::default()),
            (
                "Breed selected pair".into(),
                NavigateTo::BreedSelectedPair,
                false
            )
        );
        assert!(breeding_text(&view, PetSelection::default()).contains("at least 7 days old"));
    }

    #[test]
    fn selected_pets_drive_care_and_breeding_copy() {
        let mut view = view();
        let first = view.summary.as_ref().unwrap().pets[0].clone();
        let mut second = first.clone();
        second.id = "second-pet".into();
        second.species = "mossback".into();
        let mut third = first.clone();
        third.id = "third-pet".into();
        third.species = "aquafin".into();
        view.summary.as_mut().unwrap().pets = vec![first, second.clone(), third.clone()];
        let selection = PetSelection {
            primary: 2,
            secondary: 1,
        };

        assert!(pet_status_text(&view, selection).starts_with("Sparky the Aquafin"));
        let breeding = breeding_text(&view, selection);
        assert!(breeding.contains("Parent A: Sparky (Aquafin)"));
        assert!(breeding.contains("Parent B: Sparky (Mossback)"));
    }

    #[test]
    fn battle_actions_use_authored_moves_and_stay_inside_energy_budgets() {
        let mut view = view();
        let ids = [
            PetId::derive(WorldSeed(3), Tick::ZERO, 1),
            PetId::derive(WorldSeed(3), Tick::ZERO, 2),
        ];
        let mut companion = view.summary.as_ref().unwrap().pets[0].clone();
        view.summary.as_mut().unwrap().pets[0].id = ids[0].to_string();
        companion.id = ids[1].to_string();
        view.summary.as_mut().unwrap().pets.push(companion);
        let battle = BattleId::derive(WorldSeed(3), Tick::ZERO, 3);
        view.summary.as_mut().unwrap().active_battles = vec![BattleDto {
            id: battle.to_string(),
            combatant_ids: [ids[0].to_string(), ids[1].to_string()],
            health: [100, 100],
            energy: [100, 100],
            turn: 0,
        }];
        let moves = vec![
            BattleMoveDto {
                id: "tackle".into(),
                power: 30,
                accuracy_milli: 95_000,
                energy_cost: 8,
            },
            BattleMoveDto {
                id: "warm_nuzzle".into(),
                power: 20,
                accuracy_milli: 100_000,
                energy_cost: 5,
            },
        ];
        view.battle_setup = Some(BattleSetupDto {
            pets: ids
                .into_iter()
                .map(|id| BattlePetDto {
                    id: id.to_string(),
                    name: "Sparky".into(),
                    species: "emberling".into(),
                    moves: moves.clone(),
                })
                .collect(),
        });

        let (resolved_battle, actions) = battle_actions(&view, BattleTactic::Power).unwrap();

        assert_eq!(resolved_battle, battle);
        assert_eq!(actions.len(), 12);
        assert!(
            actions
                .iter()
                .all(|pair| pair[0].move_id.to_string() == "tackle")
        );
        assert_eq!(
            battle_primary_action(&view),
            ("Choose tactic".into(), NavigateTo::Noop)
        );
        let (_, precision) = battle_actions(&view, BattleTactic::Precision).unwrap();
        assert!(
            precision
                .iter()
                .all(|pair| pair[0].move_id.to_string() == "warm_nuzzle")
        );
        let (_, efficient) = battle_actions(&view, BattleTactic::Efficient).unwrap();
        assert!(
            efficient
                .iter()
                .all(|pair| pair[0].move_id.to_string() == "warm_nuzzle")
        );
    }

    #[test]
    fn save_slots_mark_the_active_slot_and_cycle_deterministically() {
        let mut view = view();
        let slots = [
            domain::identifiers::SaveSlotId::derive(WorldSeed(4), Tick::ZERO, 1),
            domain::identifiers::SaveSlotId::derive(WorldSeed(4), Tick::ZERO, 2),
        ];
        view.save_slots = slots
            .iter()
            .enumerate()
            .map(|(index, slot)| SaveSlotDto {
                id: slot.to_string(),
                schema_version: 3,
                updated_at_unix: index as i64,
            })
            .collect();

        assert_eq!(next_save_slot(&view, slots[0]), Some(slots[1]));
        let rendered = save_slots_text(&view, slots[0]);
        assert!(rendered.contains("active"));
        assert!(rendered.contains("available"));
    }
}
