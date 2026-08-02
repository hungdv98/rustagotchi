use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use application::ports::{SaveRepository, SlotSummary};
use application::{Command, GameEngine, GameState, SaveError, SaveMetadata};
use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use bevy::window::{PresentMode, WindowPlugin, WindowResolution};
use content::ContentLoader;
use domain::genetics::{Ancestry, GeneticProfile, Parentage};
use domain::identifiers::{PetId, SaveSlotId, SpeciesId};
use domain::pet::{Experience, Personality, Pet, PetName};
use domain::rng::WorldSeed;
use domain::time::{Tick, UnixSeconds};
use presentation::{
    ActiveSaveSlot, AppState, BattleResultView, EngineResource, GameViewModel,
    RustagotchiPresentationPlugin,
};
use simulation::SimConfig;

const WIDTH: u32 = 1_100;
const HEIGHT: u32 = 720;
const PET_COUNT: usize = 8;

fn main() -> Result<()> {
    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    let screen = VerificationScreen::from_env()?;
    let output = output_path(screen);
    let content = ContentLoader::load(&assets).context("loading verification content")?;
    let config = SimConfig::from_ron(
        &std::fs::read_to_string(assets.join("config/simulation.ron"))
            .context("reading verification simulation configuration")?,
    )
    .context("validating verification simulation configuration")?;
    let seed = WorldSeed(0x5245_4e44_4552);
    let slot = SaveSlotId::derive(seed, Tick::ZERO, 1);
    let mut state = GameState::new(seed, UnixSeconds::EPOCH);
    if !matches!(screen, VerificationScreen::PetCreation) {
        for ordinal in 1..=PET_COUNT as u64 {
            state.simulation.insert_pet(founder(seed, ordinal));
        }
    }
    state.next_entity_ordinal = if matches!(screen, VerificationScreen::PetCreation) {
        1
    } else {
        PET_COUNT as u64 + 1
    };
    let mut verification_view = GameViewModel::default();
    if matches!(screen, VerificationScreen::BattleResult) {
        let winner = state
            .simulation
            .pets
            .values_mut()
            .next()
            .expect("verification winner exists");
        let levels_gained = winner.progression.grant(Experience(100));
        state.wallet = domain::economy::Wallet::with_balance(domain::economy::Currency(50));
        verification_view.last_battle_result = Some(BattleResultView {
            winner_name: winner.name.to_string(),
            coins_earned: 50,
            balance: 50,
            experience_earned: 100,
            level: winner.progression.level().get(),
            experience: winner.progression.experience().0,
            experience_for_next_level: domain::pet::PetProgression::required_for_next(
                winner.progression.level(),
            ),
            levels_gained,
        });
    }
    let saves: Box<dyn SaveRepository> = Box::new(VerificationRepository);
    let mut engine = GameEngine::new(state, content, config, saves);
    if matches!(screen, VerificationScreen::Battle) {
        engine
            .dispatch(Command::StartBattle {
                first: PetId::derive(seed, Tick::ZERO, 1),
                second: PetId::derive(seed, Tick::ZERO, 2),
            })
            .context("starting verification battle")?;
    }

    let mut app = App::new();
    app.insert_resource(EngineResource(engine))
        .insert_resource(verification_view)
        .insert_resource(ActiveSaveSlot(slot))
        .insert_resource(screen)
        .insert_resource(VerificationOutput(output.clone()))
        .init_resource::<VerificationDriver>()
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: assets.to_string_lossy().into_owned(),
                    ..default()
                })
                .set(RenderPlugin {
                    synchronous_pipeline_compilation: true,
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Rustagotchi render verification".into(),
                        resolution: WindowResolution::new(WIDTH, HEIGHT)
                            .with_scale_factor_override(1.0),
                        present_mode: PresentMode::AutoNoVsync,
                        resizable: false,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(RustagotchiPresentationPlugin)
        .add_systems(Update, drive_verification);

    let exit = app.run();
    ensure!(
        exit == AppExit::Success,
        "render verifier exited unsuccessfully"
    );
    let metadata = std::fs::metadata(&output)
        .with_context(|| format!("reading rendered artifact {}", output.display()))?;
    ensure!(
        metadata.len() > 10_000,
        "rendered artifact is unexpectedly small"
    );
    println!("render verification passed: {}", output.display());
    Ok(())
}

fn founder(seed: WorldSeed, ordinal: u64) -> Pet {
    let species = match ordinal % 3 {
        1 => "emberling",
        2 => "stormling",
        _ => "aquafin",
    };
    let parentage = if ordinal <= 2 {
        Parentage::Founder
    } else {
        Parentage::Bred {
            parents: [
                PetId::derive(seed, Tick::ZERO, 1),
                PetId::derive(seed, Tick::ZERO, 2),
            ],
        }
    };
    let mut pet = Pet::hatch(
        PetId::derive(seed, Tick::ZERO, ordinal),
        SpeciesId::new(species).expect("built-in species id is valid"),
        GeneticProfile::new(),
        Personality::neutral(),
        parentage,
        Ancestry::founder(),
        Tick::ZERO,
    );
    pet.rename(PetName::new(format!("Pet {ordinal}")).expect("verification name is valid"));
    pet
}

fn output_path(screen: VerificationScreen) -> PathBuf {
    std::env::var_os("RUSTAGOTCHI_RENDER_OUTPUT").map_or_else(
        || {
            PathBuf::from(format!(
                "target/render-verification/rustagotchi-{}.png",
                screen.slug()
            ))
        },
        PathBuf::from,
    )
}

#[derive(Clone, Copy, Debug, Resource)]
enum VerificationScreen {
    Home,
    PetStatus,
    PetCreation,
    Battle,
    BattleResult,
}

impl VerificationScreen {
    fn from_env() -> Result<Self> {
        match std::env::var("RUSTAGOTCHI_RENDER_SCREEN")
            .unwrap_or_else(|_| "home".into())
            .as_str()
        {
            "home" => Ok(Self::Home),
            "pet-status" => Ok(Self::PetStatus),
            "pet-creation" => Ok(Self::PetCreation),
            "battle" => Ok(Self::Battle),
            "battle-result" => Ok(Self::BattleResult),
            value => anyhow::bail!(
                "unsupported RUSTAGOTCHI_RENDER_SCREEN `{value}`; use `home`, `pet-status`, `pet-creation`, `battle` or `battle-result`"
            ),
        }
    }

    const fn state(self) -> AppState {
        match self {
            Self::Home => AppState::Home,
            Self::PetStatus => AppState::PetStatus,
            Self::PetCreation => AppState::PetCreation,
            Self::Battle => AppState::Battle,
            Self::BattleResult => AppState::Battle,
        }
    }

    const fn slug(self) -> &'static str {
        match self {
            Self::Home => "home",
            Self::PetStatus => "pet-status",
            Self::PetCreation => "pet-creation",
            Self::Battle => "battle",
            Self::BattleResult => "battle-result",
        }
    }
}

#[derive(Debug, Resource)]
struct VerificationOutput(PathBuf);

#[derive(Debug, Default, Resource)]
struct VerificationDriver {
    screen_frames: u16,
    requested: bool,
}

fn drive_verification(
    mut commands: Commands<'_, '_>,
    state: Res<'_, State<AppState>>,
    mut next: ResMut<'_, NextState<AppState>>,
    screen: Res<'_, VerificationScreen>,
    asset_server: Res<'_, AssetServer>,
    portraits: Query<'_, '_, &ImageNode>,
    mut driver: ResMut<'_, VerificationDriver>,
) {
    if *state.get() == AppState::MainMenu {
        next.set(screen.state());
        return;
    }
    if *state.get() != screen.state() {
        return;
    }
    driver.screen_frames += 1;
    assert!(
        driver.screen_frames < 600,
        "render verification timed out waiting for the target screen"
    );
    if driver.requested {
        return;
    }
    let images: Vec<_> = portraits.iter().map(|node| node.image.id()).collect();
    let home_assets_pending = matches!(*screen, VerificationScreen::Home)
        && (images.len() != PET_COUNT
            || !images
                .iter()
                .all(|image| asset_server.is_loaded_with_dependencies(*image)));
    if home_assets_pending || driver.screen_frames < 10 {
        return;
    }
    driver.requested = true;
    commands
        .spawn(Screenshot::primary_window())
        .observe(validate_screenshot);
}

fn validate_screenshot(
    captured: On<'_, '_, ScreenshotCaptured>,
    output: Res<'_, VerificationOutput>,
    screen: Res<'_, VerificationScreen>,
    mut exit: MessageWriter<'_, AppExit>,
) {
    assert_eq!(captured.image.width(), WIDTH);
    assert_eq!(captured.image.height(), HEIGHT);
    let image = captured
        .image
        .clone()
        .try_into_dynamic()
        .expect("captured frame converts to an image")
        .to_rgba8();
    let pixels = u64::from(WIDTH) * u64::from(HEIGHT);
    let foreground = image
        .pixels()
        .filter(|pixel| {
            let [red, green, blue, _] = pixel.0;
            red.abs_diff(14) > 12 || green.abs_diff(18) > 12 || blue.abs_diff(26) > 12
        })
        .count() as u64;
    let ember = image
        .pixels()
        .filter(|pixel| {
            let [red, green, blue, _] = pixel.0;
            red > 170 && green > 65 && green < 225 && blue < 95
        })
        .count();
    let healthy = image
        .pixels()
        .filter(|pixel| {
            let [red, green, blue, _] = pixel.0;
            red < 110 && green > 130 && blue < 170
        })
        .count();
    let storm = image
        .pixels()
        .filter(|pixel| {
            let [red, green, blue, _] = pixel.0;
            red > 45 && red < 155 && green < 125 && blue > 120
        })
        .count();
    let aqua = image
        .pixels()
        .filter(|pixel| {
            let [red, green, blue, _] = pixel.0;
            red < 100 && green > 120 && blue > 125
        })
        .count();
    if let Some(parent) = output.0.parent() {
        std::fs::create_dir_all(parent).expect("create render verification output directory");
    }
    image
        .save(&output.0)
        .expect("save render verification artifact");
    println!(
        "captured {foreground} foreground, {ember} ember, {storm} storm, {aqua} aqua and {healthy} healthy pixels out of {pixels}"
    );
    assert!(
        foreground > pixels / 25,
        "captured frame is effectively blank"
    );
    match *screen {
        VerificationScreen::Home => {
            assert!(ember > 2_500, "captured frame is missing Emberling artwork");
            assert!(storm > 1_000, "captured frame is missing Stormling artwork");
            assert!(aqua > 1_000, "captured frame is missing Aquafin artwork");
        }
        VerificationScreen::PetStatus => {
            assert!(
                healthy > 1_000,
                "captured pet status is missing healthy need-meter feedback"
            );
        }
        VerificationScreen::Battle | VerificationScreen::BattleResult => {}
        VerificationScreen::PetCreation => {}
    }
    exit.write(AppExit::Success);
}

#[derive(Debug)]
struct VerificationRepository;

impl SaveRepository for VerificationRepository {
    fn list_slots(&self) -> Result<Vec<SlotSummary>, SaveError> {
        Ok(Vec::new())
    }

    fn save(&self, _slot: SaveSlotId, state: &GameState) -> Result<SaveMetadata, SaveError> {
        Ok(state.metadata.clone())
    }

    fn load(&self, _slot: SaveSlotId) -> Result<GameState, SaveError> {
        Err(SaveError::NotFound)
    }

    fn delete(&self, _slot: SaveSlotId) -> Result<(), SaveError> {
        Ok(())
    }
}
