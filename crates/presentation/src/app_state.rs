use bevy::prelude::*;

#[derive(States, Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum AppState {
    #[default]
    Loading,
    MainMenu,
    SaveSlots,
    ResetConfirmation,
    PetCreation,
    Home,
    PetStatus,
    PetRelease,
    Inventory,
    Breeding,
    Battle,
}
