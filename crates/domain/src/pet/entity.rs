//! The pet aggregate.

use serde::{Deserialize, Serialize};

use super::activity::ActivityState;
use super::emotions::{EmotionalState, Mood};
use super::habits::Habits;
use super::health::HealthState;
use super::life_stage::LifeStage;
use super::memories::Memories;
use super::name::PetName;
use super::needs::{NeedKind, NeedValue, PhysicalNeeds};
use super::personality::Personality;
use super::progression::PetProgression;
use super::relationships::Relationships;
use super::stats::Stats;
use crate::error::DomainError;
use crate::events::{DeathCause, DomainEvents, PetEvent};
use crate::genetics::{Ancestry, GeneticProfile, Parentage};
use crate::identifiers::{PetId, SpeciesId};
use crate::time::Tick;
use crate::units::Milli;

/// Everything about one pet.
///
/// The aggregate root. Fields are public because this is a data model, not an
/// encapsulation exercise — the simulation systems in Phase 3 need direct access
/// to write efficiently. What is *not* public is any way to break an invariant:
/// the field types themselves refuse out-of-range values, so `pet.needs.hunger`
/// can be assigned freely and still cannot leave `0.000..=100.000`.
///
/// The few operations that span several fields, and therefore have an invariant
/// no single type can enforce, are methods here — and they return events rather
/// than calling out to anything (ADR 0009).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pet {
    pub id: PetId,
    #[serde(default)]
    pub name: PetName,
    pub species: SpeciesId,
    pub stage: LifeStage,
    pub needs: PhysicalNeeds,
    pub emotions: EmotionalState,
    pub personality: Personality,
    pub genetics: GeneticProfile,
    pub stats: Stats,
    pub progression: PetProgression,
    pub health: HealthState,
    pub activity: ActivityState,
    pub memories: Memories,
    pub habits: Habits,
    pub relationships: Relationships,
    pub parentage: Parentage,
    pub ancestry: Ancestry,
}

impl Pet {
    /// Satisfaction at or below which a need counts as critical.
    pub const CRITICAL_NEED: NeedValue = NeedValue::new(Milli(10_000));

    /// Creates a pet at [`LifeStage::Hatchling`].
    ///
    /// Everything derived — emotions, activity — is computed rather than passed
    /// in, so there is no way to hatch a pet whose mood disagrees with its needs.
    #[must_use]
    pub fn hatch(
        id: PetId,
        species: SpeciesId,
        genetics: GeneticProfile,
        personality: Personality,
        parentage: Parentage,
        ancestry: Ancestry,
        now: Tick,
    ) -> Self {
        let needs = PhysicalNeeds::full();
        Pet {
            id,
            name: PetName::default(),
            species,
            stage: LifeStage::Hatchling,
            emotions: EmotionalState::from_needs(&needs),
            needs,
            personality,
            genetics,
            stats: Stats::default(),
            progression: PetProgression::born_at(now),
            health: HealthState::healthy(),
            activity: ActivityState::idle_since(now),
            memories: Memories::new(),
            habits: Habits::new(),
            relationships: Relationships::new(),
            parentage,
            ancestry,
        }
    }

    #[must_use]
    pub fn is_alive(&self) -> bool {
        self.health.is_alive()
    }

    pub fn rename(&mut self, name: PetName) {
        self.name = name;
    }

    #[must_use]
    pub fn mood(&self) -> Mood {
        Mood::of(&self.emotions)
    }

    /// Needs at or below [`Self::CRITICAL_NEED`], in [`NeedKind::ALL`] order.
    #[must_use]
    pub fn critical_needs(&self) -> smallvec::SmallVec<[NeedKind; 6]> {
        self.needs
            .iter()
            .filter(|(_, value)| value.is_at_or_below(Self::CRITICAL_NEED))
            .map(|(kind, _)| kind)
            .collect()
    }

    /// Whether this pet may currently breed at all.
    ///
    /// The coarse gate only — species compatibility, cooldowns and lineage are
    /// checked by `domain::breeding` in Phase 5.
    #[must_use]
    pub fn can_breed(&self) -> bool {
        self.is_alive() && self.stage.can_breed() && !self.health.is_ill()
    }

    /// Sets a need, emitting the change and any threshold crossing.
    ///
    /// The threshold events are the reason this is a method rather than a field
    /// assignment: "became critical" fires on entry only, and knowing that
    /// requires both the old and new values. A caller writing
    /// `pet.needs.hunger = x` directly gets no events, which is correct — it is
    /// the callers that want notifications that come through here.
    pub fn set_need(&mut self, kind: NeedKind, value: NeedValue, now: Tick) -> DomainEvents {
        let previous = self.needs.get(kind);
        if previous == value {
            return DomainEvents::new();
        }
        self.needs.set(kind, value);

        let mut events = DomainEvents::new();
        events.push(
            PetEvent::NeedChanged {
                pet: self.id,
                need: kind,
                from: previous,
                to: value,
            }
            .into(),
        );

        let was_critical = previous.is_at_or_below(Self::CRITICAL_NEED);
        let is_critical = value.is_at_or_below(Self::CRITICAL_NEED);
        match (was_critical, is_critical) {
            (false, true) => events.push(
                PetEvent::NeedBecameCritical {
                    pet: self.id,
                    need: kind,
                    at: now,
                }
                .into(),
            ),
            (true, false) => {
                events.push(
                    PetEvent::NeedRecovered {
                        pet: self.id,
                        need: kind,
                        at: now,
                    }
                    .into(),
                );
            }
            _ => {}
        }
        events
    }

    /// Advances the life stage.
    ///
    /// # Errors
    ///
    /// [`DomainError::LifeStageRegression`] if `target` is earlier than the
    /// current stage, or [`DomainError::PetIsDead`] if the pet has died.
    pub fn advance_stage(
        &mut self,
        target: LifeStage,
        now: Tick,
    ) -> Result<DomainEvents, DomainError> {
        if !self.is_alive() {
            return Err(DomainError::PetIsDead {
                pet: self.id,
                action: "grow",
            });
        }
        let previous = self.stage;
        self.stage.advance_to(target)?;

        let mut events = DomainEvents::new();
        if previous != self.stage {
            events.push(
                PetEvent::LifeStageAdvanced {
                    pet: self.id,
                    from: previous,
                    to: self.stage,
                    at: now,
                }
                .into(),
            );
        }
        Ok(events)
    }

    /// Kills the pet, emitting the death exactly once.
    ///
    /// Idempotent: a pet already dead emits nothing. Without that, a starvation
    /// check running every tick would emit a death event every tick after the
    /// first, and the notification system would tell the player their pet had
    /// died several thousand times.
    pub fn die(&mut self, cause: DeathCause, now: Tick) -> DomainEvents {
        if !self.is_alive() {
            return DomainEvents::new();
        }
        self.health.damage(Milli(i32::MAX));

        let mut events = DomainEvents::new();
        events.push(
            PetEvent::Died {
                pet: self.id,
                cause,
                at: now,
            }
            .into(),
        );
        events
    }

    /// Recomputes emotions from the current needs.
    ///
    /// Derived rather than stored-and-updated, so the live and offline paths
    /// cannot disagree about a pet's mood.
    pub fn refresh_emotions(&mut self) {
        self.emotions = EmotionalState::from_needs(&self.needs);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::DomainEvent;
    use crate::rng::WorldSeed;

    fn a_pet() -> Pet {
        Pet::hatch(
            PetId::derive(WorldSeed(1), Tick::ZERO, 1),
            SpeciesId::new("emberling").expect("valid species"),
            GeneticProfile::new(),
            Personality::neutral(),
            Parentage::Founder,
            Ancestry::founder(),
            Tick(100),
        )
    }

    fn pet_events(events: &DomainEvents) -> Vec<PetEvent> {
        events
            .iter()
            .filter_map(|event| match event {
                DomainEvent::Pet(pet_event) => Some(pet_event.clone()),
                DomainEvent::WorldReset => None,
                DomainEvent::Breeding(_) => None,
                DomainEvent::Combat(_) => None,
            })
            .collect()
    }

    #[test]
    fn a_hatched_pet_is_alive_satisfied_and_idle() {
        let pet = a_pet();
        assert!(pet.is_alive());
        assert_eq!(pet.stage, LifeStage::Hatchling);
        assert!(pet.needs.iter().all(|(_, value)| value.is_full()));
        assert!(pet.critical_needs().is_empty());
        assert_eq!(pet.progression.birth_tick(), Tick(100));
        assert!(
            pet.mood() >= Mood::Happy,
            "a fully satisfied pet should be happy"
        );
    }

    #[test]
    fn a_hatchling_cannot_breed() {
        assert!(!a_pet().can_breed());
    }

    #[test]
    fn an_adult_can_breed_unless_ill_or_dead() {
        let mut pet = a_pet();
        pet.advance_stage(LifeStage::Adult, Tick(200))
            .expect("grow up");
        assert!(pet.can_breed());

        pet.health.contract(
            crate::identifiers::IllnessId::new("sniffles").expect("valid"),
            crate::pet::health::Severity::Mild,
            Tick(201),
        );
        assert!(!pet.can_breed(), "an ill pet must not breed");

        pet.health
            .cure(&crate::identifiers::IllnessId::new("sniffles").expect("valid"));
        pet.die(DeathCause::OldAge, Tick(300));
        assert!(!pet.can_breed());
    }

    #[test]
    fn setting_a_need_emits_the_change() {
        let mut pet = a_pet();
        let events = pet.set_need(NeedKind::Hunger, NeedValue::new(Milli(50_000)), Tick(101));
        assert_eq!(events.len(), 1);
        assert!(matches!(
            pet_events(&events)[0],
            PetEvent::NeedChanged { .. }
        ));
    }

    #[test]
    fn setting_a_need_to_its_current_value_emits_nothing() {
        let mut pet = a_pet();
        let events = pet.set_need(NeedKind::Hunger, NeedValue::FULL, Tick(101));
        assert!(events.is_empty(), "a no-op must not produce an event");
    }

    #[test]
    fn crossing_into_the_critical_band_emits_once_not_every_tick() {
        let mut pet = a_pet();

        let entering = pet.set_need(NeedKind::Hunger, NeedValue::new(Milli(5_000)), Tick(101));
        assert!(
            pet_events(&entering)
                .iter()
                .any(|event| matches!(event, PetEvent::NeedBecameCritical { .. })),
            "entry into the critical band must be announced"
        );

        // Still critical, but lower: the change is reported, the crossing is not.
        let deeper = pet.set_need(NeedKind::Hunger, NeedValue::new(Milli(1_000)), Tick(102));
        assert!(
            !pet_events(&deeper)
                .iter()
                .any(|event| matches!(event, PetEvent::NeedBecameCritical { .. })),
            "staying critical must not re-announce"
        );
    }

    #[test]
    fn recovering_out_of_the_critical_band_is_announced() {
        let mut pet = a_pet();
        pet.set_need(NeedKind::Hunger, NeedValue::new(Milli(1_000)), Tick(101));
        let recovery = pet.set_need(NeedKind::Hunger, NeedValue::FULL, Tick(102));
        assert!(
            pet_events(&recovery)
                .iter()
                .any(|event| matches!(event, PetEvent::NeedRecovered { .. }))
        );
    }

    #[test]
    fn critical_needs_are_listed_in_a_fixed_order() {
        let mut pet = a_pet();
        pet.set_need(NeedKind::Fun, NeedValue::EMPTY, Tick(101));
        pet.set_need(NeedKind::Energy, NeedValue::EMPTY, Tick(101));
        assert_eq!(
            pet.critical_needs().to_vec(),
            [NeedKind::Energy, NeedKind::Fun]
        );
    }

    #[test]
    fn advancing_the_stage_emits_once_and_refuses_to_regress() {
        let mut pet = a_pet();
        let events = pet
            .advance_stage(LifeStage::Child, Tick(200))
            .expect("advance");
        assert_eq!(events.len(), 1);

        // Advancing to the same stage is allowed but silent.
        let repeat = pet
            .advance_stage(LifeStage::Child, Tick(201))
            .expect("no-op advance");
        assert!(repeat.is_empty());

        let error = pet.advance_stage(LifeStage::Egg, Tick(202)).unwrap_err();
        assert!(matches!(error, DomainError::LifeStageRegression { .. }));
        assert_eq!(pet.stage, LifeStage::Child);
    }

    #[test]
    fn a_dead_pet_cannot_grow() {
        let mut pet = a_pet();
        pet.die(DeathCause::OldAge, Tick(300));
        let error = pet.advance_stage(LifeStage::Adult, Tick(301)).unwrap_err();
        assert!(matches!(error, DomainError::PetIsDead { .. }));
    }

    #[test]
    fn death_is_emitted_exactly_once() {
        let mut pet = a_pet();
        let first = pet.die(
            DeathCause::Neglect {
                need: NeedKind::Hunger,
            },
            Tick(300),
        );
        assert_eq!(first.len(), 1);
        assert!(!pet.is_alive());

        // A per-tick starvation check must not announce the death again.
        for tick in 301..400 {
            assert!(pet.die(DeathCause::OldAge, Tick(tick)).is_empty());
        }
    }

    #[test]
    fn emotions_track_the_needs_they_are_refreshed_from() {
        let mut pet = a_pet();
        assert!(pet.mood() >= Mood::Happy);

        for kind in NeedKind::ALL {
            pet.set_need(kind, NeedValue::EMPTY, Tick(101));
        }
        pet.refresh_emotions();
        assert_eq!(pet.mood(), Mood::Miserable);
    }

    #[test]
    fn a_pet_survives_a_serde_round_trip() {
        let mut pet = a_pet();
        pet.set_need(NeedKind::Hunger, NeedValue::new(Milli(42_000)), Tick(101));
        pet.memories.record(
            super::super::memories::MemoryKind::Fed,
            Tick(101),
            Milli(50_000),
        );
        pet.habits
            .reinforce(super::super::activity::Activity::Playing, Milli(1_000));

        let encoded = serde_json::to_string(&pet).expect("serialize");
        let decoded: Pet = serde_json::from_str(&encoded).expect("deserialize");
        assert_eq!(pet, decoded);
    }

    #[test]
    fn a_legacy_pet_without_a_name_receives_the_migration_default() {
        let mut encoded = serde_json::to_value(a_pet()).expect("serialize");
        encoded.as_object_mut().unwrap().remove("name");

        let decoded: Pet = serde_json::from_value(encoded).expect("deserialize legacy pet");

        assert_eq!(decoded.name.as_str(), "Companion");
    }
}
