//! Bevy presentation layer: screens, widgets, animation, audio, view models.
//!
//! The engine boundary is strict: `application::GameEngine` lives in a Bevy
//! `Resource`, gameplay systems send `Command`s through an `EventWriter`, and a
//! single `dispatch_commands` system drains them and re-emits the resulting
//! `DomainEvent`s as Bevy events. **No Bevy system mutates `GameState`
//! directly.** That is what keeps the dependency direction honest at runtime
//! and not merely at compile time.
//!
//! Presentation reads DTOs from `application::dto`, never domain types. This is
//! also the only crate in the workspace where `f32`/`f64` are permitted --
//! positions, easing and audio are not simulation state.
//!
//! Phase 6 builds: `app_state`, `resources`, `events`, `view_models`,
//! `plugins/*`, `screens/*`, `widgets/*`, `animation/*`, `audio/*`.

mod animation;
mod app_state;
mod events;
mod feedback;
mod performance;
mod plugin;
mod resources;
mod screens;
mod view_models;

pub use app_state::AppState;
pub use events::{AppCommand, CommandFailed, DomainEventMessage, SwitchSaveSlot};
pub use feedback::{FeedbackCue, PresentationPreferences};
pub use performance::{EIGHT_PET_FRAME_BUDGET, FramePerformanceSnapshot};
pub use plugin::RustagotchiPresentationPlugin;
pub use resources::{ActiveSaveSlot, EngineResource, PetSelection};
pub use view_models::{BattleResultView, GameViewModel};
