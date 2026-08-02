//! The pet aggregate.
//!
//! [`entity::Pet`] is the root; every other module here is one of its parts.
//!
//! Two conventions run through all of them:
//!
//! * **Invariants live in the types.** `NeedValue`, `StatValue`, `TraitValue`
//!   and `Affinity` all clamp on construction, so the aggregate's fields can be
//!   public without any of them being able to hold an impossible value.
//! * **Collections are bounded.** Memories, habits, relationships and ailments
//!   all have a cap and a documented eviction rule. A pet simulated for a year
//!   receives half a million ticks; anything unbounded becomes a save that grows
//!   forever.

pub mod activity;
pub mod emotions;
pub mod entity;
pub mod habits;
pub mod health;
pub mod life_stage;
pub mod memories;
pub mod name;
pub mod needs;
pub mod personality;
pub mod progression;
pub mod relationships;
pub mod stats;

pub use activity::{Activity, ActivityState};
pub use emotions::{Emotion, EmotionalState, Intensity, Mood};
pub use entity::Pet;
pub use habits::Habits;
pub use health::{Ailment, HealthState, Severity};
pub use life_stage::LifeStage;
pub use memories::{Memories, Memory, MemoryKind};
pub use name::{MAX_PET_NAME_CHARS, PetName, PetNameError};
pub use needs::{NeedKind, NeedValue, PhysicalNeeds};
pub use personality::{Personality, TraitKind, TraitValue};
pub use progression::{Experience, Level, PetProgression};
pub use relationships::{Affinity, Relationships};
pub use stats::{StatKind, StatValue, Stats};
