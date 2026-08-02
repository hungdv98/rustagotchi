use content::ContentRegistry;
use domain::combat::{BattleAction, BattleState, CombatEvent, Combatant};
use domain::economy::Currency;
use domain::events::{BreedingEvent, DomainEvent, PetEvent};
use domain::genetics::{Ancestry, GeneticProfile, Parentage};
use domain::identifiers::{BattleId, EggId, ItemId, PetId, SaveSlotId, SpeciesId};
use domain::inventory::ItemCategory;
use domain::pet::{Experience, NeedKind, NeedValue, Personality, Pet, PetName};
use domain::rng::{RngStream, rng_for};
use domain::time::UnixSeconds;

use crate::ports::SaveRepository;
use crate::queries::{ContentQuery, Query};
use crate::{AppError, GameState, SaveError};

/// Founder pets are adopted directly; later generations come from breeding.
pub const BASE_ADOPTED_PETS: usize = 2;
pub const MAX_ADOPTED_PETS: usize = 8;
pub const MAX_ADOPTION_SLOT_UPGRADES: u8 = (MAX_ADOPTED_PETS - BASE_ADOPTED_PETS) as u8;
const ADOPTION_SLOT_COSTS: [i64; MAX_ADOPTION_SLOT_UPGRADES as usize] =
    [100, 200, 400, 800, 1_600, 3_200];

#[must_use]
pub const fn adopted_pet_capacity(upgrades: u8) -> usize {
    let bounded = if upgrades > MAX_ADOPTION_SLOT_UPGRADES {
        MAX_ADOPTION_SLOT_UPGRADES
    } else {
        upgrades
    };
    BASE_ADOPTED_PETS + bounded as usize
}

#[must_use]
pub const fn next_adoption_slot_cost(upgrades: u8) -> Option<Currency> {
    if upgrades >= MAX_ADOPTION_SLOT_UPGRADES {
        None
    } else {
        Some(Currency(ADOPTION_SLOT_COSTS[upgrades as usize]))
    }
}

#[derive(Debug, Clone)]
pub enum Command {
    CreatePet {
        species: SpeciesId,
        name: PetName,
    },
    ReleasePet {
        pet: PetId,
    },
    ResetGame {
        slot: SaveSlotId,
    },
    UnlockAdoptionSlot,
    AdvanceTo {
        wall_now: UnixSeconds,
    },
    FeedPet {
        pet: PetId,
        item: ItemId,
    },
    Breed {
        first: PetId,
        second: PetId,
    },
    HatchEgg {
        egg: EggId,
    },
    StartBattle {
        first: PetId,
        second: PetId,
    },
    ResolveBattle {
        battle: BattleId,
        actions: Vec<[BattleAction; 2]>,
    },
    Save {
        slot: SaveSlotId,
    },
    Load {
        slot: SaveSlotId,
    },
}

pub struct GameEngine<R> {
    state: GameState,
    content: ContentRegistry,
    config: simulation::SimConfig,
    saves: R,
}

impl<R: SaveRepository> GameEngine<R> {
    #[must_use]
    pub fn new(
        state: GameState,
        content: ContentRegistry,
        config: simulation::SimConfig,
        saves: R,
    ) -> Self {
        Self {
            state,
            content,
            config,
            saves,
        }
    }
    #[must_use]
    pub fn state(&self) -> &GameState {
        &self.state
    }

    pub fn query<Q: Query>(&self, query: Q) -> Q::Output {
        query.execute(&self.state)
    }

    pub fn content_query<Q: ContentQuery>(&self, query: Q) -> Q::Output {
        query.execute(&self.state, &self.content)
    }

    pub fn list_save_slots(&self) -> Result<Vec<crate::dto::SaveSlotDto>, AppError> {
        Ok(self
            .saves
            .list_slots()?
            .into_iter()
            .map(|slot| crate::dto::SaveSlotDto {
                id: slot.slot.to_string(),
                schema_version: slot.schema_version,
                updated_at_unix: slot.updated_at.0,
            })
            .collect())
    }

    pub fn dispatch(&mut self, command: Command) -> Result<Vec<DomainEvent>, AppError> {
        match command {
            Command::CreatePet { species, name } => self.create_pet(species, name),
            Command::ReleasePet { pet } => self.release_pet(pet),
            Command::ResetGame { slot } => self.reset_game(slot),
            Command::UnlockAdoptionSlot => self.unlock_adoption_slot(),
            Command::AdvanceTo { wall_now } => self.advance_to(wall_now),
            Command::FeedPet { pet, item } => self.feed_pet(pet, item),
            Command::Breed { first, second } => self.breed(first, second),
            Command::HatchEgg { egg } => self.hatch_egg(egg),
            Command::StartBattle { first, second } => self.start_battle(first, second),
            Command::ResolveBattle { battle, actions } => self.resolve_battle(battle, &actions),
            Command::Save { slot } => {
                self.state.metadata.last_saved_at = self.state.clock.anchored_at();
                self.saves.save(slot, &self.state)?;
                Ok(Vec::new())
            }
            Command::Load { slot } => {
                self.state = self.saves.load(slot)?;
                Ok(Vec::new())
            }
        }
    }

    fn create_pet(
        &mut self,
        species: SpeciesId,
        name: PetName,
    ) -> Result<Vec<DomainEvent>, AppError> {
        let adopted = self
            .state
            .simulation
            .pets
            .values()
            .filter(|pet| pet.parentage.is_founder())
            .count();
        let capacity = adopted_pet_capacity(self.state.adoption_slot_upgrades);
        if adopted >= capacity {
            return Err(AppError::Domain(format!(
                "all {capacity} adoption slots are full; unlock another slot with coins"
            )));
        }
        if self.content.get_species(&species).is_none() {
            return Err(AppError::Content(format!("unknown species `{species}`")));
        }
        let tick = self.state.simulation.tick;
        let pet_id = PetId::derive(
            self.state.simulation.seed,
            tick,
            self.state.next_entity_ordinal,
        );
        let mut pet = Pet::hatch(
            pet_id,
            species.clone(),
            GeneticProfile::new(),
            Personality::neutral(),
            Parentage::Founder,
            Ancestry::founder(),
            tick,
        );
        pet.rename(name);
        let starter_food =
            ItemId::new("apple_slice").map_err(|error| AppError::Content(error.to_string()))?;
        if self.content.get_item(&starter_food).is_none() {
            return Err(AppError::Content(
                "starter food `apple_slice` is missing".into(),
            ));
        }
        if self.state.simulation.pets.is_empty() {
            self.state
                .inventory
                .add(starter_food, 5)
                .map_err(|error| AppError::Domain(error.to_string()))?;
        }
        self.state.simulation.insert_pet(pet);
        self.state.next_entity_ordinal = self.state.next_entity_ordinal.saturating_add(1);
        Ok(vec![
            PetEvent::Hatched {
                pet: pet_id,
                species,
                at: tick,
            }
            .into(),
        ])
    }

    fn release_pet(&mut self, pet_id: PetId) -> Result<Vec<DomainEvent>, AppError> {
        if !self.state.simulation.pets.contains_key(&pet_id) {
            return Err(AppError::Domain(format!("pet `{pet_id}` does not exist")));
        }
        if self.state.active_battles.values().any(|battle| {
            battle
                .combatants
                .iter()
                .any(|combatant| combatant.pet == pet_id)
        }) {
            return Err(AppError::Domain(
                "a pet in an active battle cannot be released".into(),
            ));
        }
        if self
            .state
            .eggs
            .values()
            .any(|egg| egg.parents.contains(&pet_id))
        {
            return Err(AppError::Domain(
                "a pet with an unhatched egg cannot be released".into(),
            ));
        }

        self.state.simulation.pets.remove(&pet_id);
        self.state.breeding_cooldowns.remove(&pet_id);
        for pet in self.state.simulation.pets.values_mut() {
            pet.relationships.forget(pet_id);
        }
        Ok(vec![
            PetEvent::Released {
                pet: pet_id,
                at: self.state.simulation.tick,
            }
            .into(),
        ])
    }

    fn reset_game(&mut self, slot: SaveSlotId) -> Result<Vec<DomainEvent>, AppError> {
        match self.saves.delete(slot) {
            Ok(()) | Err(SaveError::NotFound) => {}
            Err(error) => return Err(error.into()),
        }
        let seed = self.state.simulation.seed;
        let wall_now = self.state.clock.anchored_at();
        self.state = GameState::new(seed, wall_now);
        Ok(vec![DomainEvent::WorldReset])
    }

    fn unlock_adoption_slot(&mut self) -> Result<Vec<DomainEvent>, AppError> {
        let upgrades = self.state.adoption_slot_upgrades;
        let cost = next_adoption_slot_cost(upgrades)
            .ok_or_else(|| AppError::Domain("all adoption slots are already unlocked".into()))?;
        self.state.wallet.withdraw(cost).map_err(|_| {
            AppError::Domain(format!(
                "you need {} coins to unlock the next adoption slot (balance: {})",
                cost.0,
                self.state.wallet.balance().0
            ))
        })?;
        self.state.adoption_slot_upgrades = upgrades.saturating_add(1);
        Ok(Vec::new())
    }

    pub fn advance_to(&mut self, wall_now: UnixSeconds) -> Result<Vec<DomainEvent>, AppError> {
        let elapsed = self.state.clock.ticks_owed(wall_now);
        let report = simulation::offline::catch_up(
            &mut self.state.simulation,
            elapsed,
            &self.content,
            &self.config,
        );
        self.state.clock.advance_after_catch_up(
            domain::time::GameDuration::from_ticks(report.simulated_ticks),
            elapsed,
        );
        self.state.metadata.clamped_offline_ticks = self.state.simulation.clamped_offline_ticks;
        Ok(report.events)
    }

    fn feed_pet(&mut self, pet_id: PetId, item_id: ItemId) -> Result<Vec<DomainEvent>, AppError> {
        let definition = self
            .content
            .get_item(&item_id)
            .ok_or_else(|| AppError::Content(format!("unknown item `{item_id}`")))?;
        if definition.category != ItemCategory::Food {
            return Err(AppError::Domain(format!("item `{item_id}` is not food")));
        }
        if !self.state.inventory.has_at_least(&item_id, 1) {
            return Err(AppError::Domain(format!(
                "item `{item_id}` is not in inventory"
            )));
        }
        let pet = self
            .state
            .simulation
            .pets
            .get_mut(&pet_id)
            .ok_or_else(|| AppError::Domain(format!("pet `{pet_id}` does not exist")))?;
        if !pet.is_alive() {
            return Err(AppError::Domain(format!("pet `{pet_id}` is dead")));
        }
        if pet.needs.get(NeedKind::Hunger) == NeedValue::FULL {
            return Err(AppError::Domain(format!("pet `{pet_id}` is not hungry")));
        }
        let next = pet
            .needs
            .get(NeedKind::Hunger)
            .restored_by(definition.potency);
        self.state
            .inventory
            .remove(&item_id, 1)
            .map_err(|error| AppError::Domain(error.to_string()))?;
        Ok(pet
            .set_need(NeedKind::Hunger, next, self.state.simulation.tick)
            .into_vec())
    }

    fn breed(&mut self, first_id: PetId, second_id: PetId) -> Result<Vec<DomainEvent>, AppError> {
        let first = self
            .state
            .simulation
            .pet(first_id)
            .cloned()
            .ok_or_else(|| AppError::Domain(format!("pet `{first_id}` does not exist")))?;
        let second = self
            .state
            .simulation
            .pet(second_id)
            .cloned()
            .ok_or_else(|| AppError::Domain(format!("pet `{second_id}` does not exist")))?;
        let compatibility = self
            .content
            .compatibility(&first.species, &second.species)
            .ok_or_else(|| AppError::Content("species compatibility is missing".into()))?;
        let species = if first.species == second.species {
            first.species.clone()
        } else {
            let mut rng = rng_for(
                self.state.simulation.seed,
                RngStream::GeneInheritance,
                self.state.simulation.tick,
                self.state.next_entity_ordinal,
            );
            if rng.chance(1, 2) {
                first.species.clone()
            } else {
                second.species.clone()
            }
        };
        let egg = simulation::breeding::generate_egg(
            &first,
            &second,
            species,
            compatibility,
            self.state.simulation.seed,
            self.state.simulation.tick,
            self.state.next_entity_ordinal,
            [
                self.state.breeding_cooldowns.get(&first_id).copied(),
                self.state.breeding_cooldowns.get(&second_id).copied(),
            ],
            &simulation::breeding::OffspringConfig::default(),
        )
        .map_err(|error| AppError::Domain(error.to_string()))?;
        self.state.next_entity_ordinal = self.state.next_entity_ordinal.saturating_add(1);
        self.state
            .breeding_cooldowns
            .insert(first_id, self.state.simulation.tick);
        self.state
            .breeding_cooldowns
            .insert(second_id, self.state.simulation.tick);
        let event = BreedingEvent::EggCreated {
            egg: egg.id,
            parents: egg.parents,
        };
        self.state.eggs.insert(egg.id, egg);
        Ok(vec![event.into()])
    }

    fn hatch_egg(&mut self, egg_id: EggId) -> Result<Vec<DomainEvent>, AppError> {
        let egg = self
            .state
            .eggs
            .get(&egg_id)
            .cloned()
            .ok_or_else(|| AppError::Domain(format!("egg `{egg_id}` does not exist")))?;
        if !egg.is_ready(self.state.simulation.tick) {
            return Err(AppError::Domain(format!("egg `{egg_id}` is not ready")));
        }
        let pet_id = PetId::derive(
            self.state.simulation.seed,
            self.state.simulation.tick,
            self.state.next_entity_ordinal,
        );
        let pet = Pet::hatch(
            pet_id,
            egg.species,
            egg.genetics,
            Personality::neutral(),
            Parentage::Bred {
                parents: egg.parents,
            },
            egg.ancestry,
            self.state.simulation.tick,
        );
        self.state.simulation.insert_pet(pet);
        self.state.eggs.remove(&egg_id);
        self.state.next_entity_ordinal = self.state.next_entity_ordinal.saturating_add(1);
        Ok(vec![
            BreedingEvent::EggHatched {
                egg: egg_id,
                pet: pet_id,
            }
            .into(),
        ])
    }

    fn start_battle(
        &mut self,
        first_id: PetId,
        second_id: PetId,
    ) -> Result<Vec<DomainEvent>, AppError> {
        if first_id == second_id {
            return Err(AppError::Domain("a pet cannot battle itself".into()));
        }
        if self.state.active_battles.values().any(|battle| {
            battle
                .combatants
                .iter()
                .any(|combatant| combatant.pet == first_id || combatant.pet == second_id)
        }) {
            return Err(AppError::Domain(
                "one of the pets is already in an active battle".into(),
            ));
        }
        let first = self
            .state
            .simulation
            .pet(first_id)
            .ok_or_else(|| AppError::Domain(format!("pet `{first_id}` does not exist")))?;
        let second = self
            .state
            .simulation
            .pet(second_id)
            .ok_or_else(|| AppError::Domain(format!("pet `{second_id}` does not exist")))?;
        if !first.is_alive() || !second.is_alive() {
            return Err(AppError::Domain("a dead pet cannot battle".into()));
        }
        let first_species = self
            .content
            .get_species(&first.species)
            .ok_or_else(|| AppError::Content(format!("unknown species `{}`", first.species)))?;
        let second_species = self
            .content
            .get_species(&second.species)
            .ok_or_else(|| AppError::Content(format!("unknown species `{}`", second.species)))?;
        let battle_id = BattleId::derive(
            self.state.simulation.seed,
            self.state.simulation.tick,
            self.state.next_entity_ordinal,
        );
        let state = BattleState {
            id: battle_id,
            combatants: [
                Combatant::from_stats(first_id, first_species.element, first.stats),
                Combatant::from_stats(second_id, second_species.element, second.stats),
            ],
            turn: 0,
        };
        self.state.next_entity_ordinal = self.state.next_entity_ordinal.saturating_add(1);
        self.state.active_battles.insert(battle_id, state);
        Ok(vec![
            CombatEvent::BattleStarted {
                battle: battle_id,
                combatants: [first_id, second_id],
            }
            .into(),
        ])
    }

    fn resolve_battle(
        &mut self,
        battle_id: BattleId,
        actions: &[[BattleAction; 2]],
    ) -> Result<Vec<DomainEvent>, AppError> {
        let state = self
            .state
            .active_battles
            .get(&battle_id)
            .cloned()
            .ok_or_else(|| AppError::Domain(format!("battle `{battle_id}` does not exist")))?;
        let result = simulation::combat::resolve_battle(
            self.state.simulation.seed,
            state,
            actions,
            &self.content,
        )
        .map_err(|error| AppError::Domain(error.to_string()))?;
        const WIN_COINS: Currency = Currency(50);
        const WIN_EXPERIENCE: Experience = Experience(100);
        self.state
            .wallet
            .deposit(WIN_COINS)
            .map_err(|error| AppError::Domain(error.to_string()))?;
        let winner = self
            .state
            .simulation
            .pets
            .get_mut(&result.winner)
            .ok_or_else(|| AppError::Domain("battle winner no longer exists".into()))?;
        let levels_gained = winner.progression.grant(WIN_EXPERIENCE);
        self.state.active_battles.remove(&battle_id);
        let mut events = result
            .events
            .into_iter()
            .map(DomainEvent::from)
            .collect::<Vec<_>>();
        events.push(
            CombatEvent::BattleWon {
                battle: battle_id,
                winner: result.winner,
                loser: result.loser,
                coins: WIN_COINS.0,
                experience: WIN_EXPERIENCE.0,
                levels_gained,
            }
            .into(),
        );
        Ok(events)
    }
}

impl<R> std::fmt::Debug for GameEngine<R> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("GameEngine")
            .field("state", &self.state)
            .field("content_pack", &self.content.pack_id())
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::SlotSummary;
    use crate::{SaveError, SaveMetadata};
    use domain::genetics::{Ancestry, GeneticProfile, Parentage};
    use domain::identifiers::SpeciesId;
    use domain::pet::{NeedValue, Personality, Pet};
    use domain::rng::WorldSeed;
    use domain::time::Tick;

    #[derive(Debug)]
    struct NoSaves;
    impl SaveRepository for NoSaves {
        fn list_slots(&self) -> Result<Vec<SlotSummary>, SaveError> {
            Ok(Vec::new())
        }
        fn save(&self, _: SaveSlotId, _: &GameState) -> Result<SaveMetadata, SaveError> {
            Err(SaveError::Storage("disabled".into()))
        }
        fn load(&self, _: SaveSlotId) -> Result<GameState, SaveError> {
            Err(SaveError::NotFound)
        }
        fn delete(&self, _: SaveSlotId) -> Result<(), SaveError> {
            Err(SaveError::NotFound)
        }
    }

    #[test]
    fn creating_a_founder_pet_is_deterministic_and_emits_an_event() {
        let seed = WorldSeed(9);
        let state = GameState::new(seed, UnixSeconds(0));
        let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let content = content::ContentLoader::load(&assets).unwrap();
        let config = simulation::SimConfig::from_ron(
            &std::fs::read_to_string(assets.join("config/simulation.ron")).unwrap(),
        )
        .unwrap();
        let mut engine = GameEngine::new(state, content, config, NoSaves);
        assert!(engine.list_save_slots().unwrap().is_empty());

        let events = engine
            .dispatch(Command::CreatePet {
                species: SpeciesId::new("emberling").unwrap(),
                name: PetName::new("Sparky").unwrap(),
            })
            .unwrap();

        let expected = PetId::derive(seed, Tick::ZERO, 1);
        assert!(engine.state.simulation.pets.contains_key(&expected));
        assert_eq!(
            engine.state.simulation.pet(expected).unwrap().name.as_str(),
            "Sparky"
        );
        assert_eq!(
            engine
                .state
                .inventory
                .count(&ItemId::new("apple_slice").unwrap()),
            5
        );
        assert!(matches!(
            &events[0],
            DomainEvent::Pet(PetEvent::Hatched { pet, .. }) if *pet == expected
        ));
        assert_eq!(engine.state.next_entity_ordinal, 2);
        assert!(
            engine
                .dispatch(Command::FeedPet {
                    pet: expected,
                    item: ItemId::new("apple_slice").unwrap(),
                })
                .is_err()
        );
        assert_eq!(
            engine
                .state
                .inventory
                .count(&ItemId::new("apple_slice").unwrap()),
            5
        );
        engine
            .dispatch(Command::AdvanceTo {
                wall_now: UnixSeconds(3_600),
            })
            .unwrap();
        assert_eq!(engine.state.simulation.tick, Tick(60));
        engine
            .dispatch(Command::FeedPet {
                pet: expected,
                item: ItemId::new("apple_slice").unwrap(),
            })
            .unwrap();
        assert_eq!(
            engine
                .state
                .inventory
                .count(&ItemId::new("apple_slice").unwrap()),
            4
        );
        engine
            .dispatch(Command::CreatePet {
                species: SpeciesId::new("stormling").unwrap(),
                name: PetName::new("Bolt").unwrap(),
            })
            .unwrap();
        assert_eq!(engine.state.simulation.pets.len(), 2);
        assert_eq!(
            engine
                .state
                .inventory
                .count(&ItemId::new("apple_slice").unwrap()),
            4
        );
        assert!(
            engine
                .dispatch(Command::CreatePet {
                    species: SpeciesId::new("emberling").unwrap(),
                    name: PetName::new("Sparky").unwrap(),
                })
                .is_err()
        );

        let events = engine
            .dispatch(Command::ReleasePet { pet: expected })
            .unwrap();
        assert!(matches!(
            &events[0],
            DomainEvent::Pet(PetEvent::Released { pet, .. }) if *pet == expected
        ));
        assert!(!engine.state.simulation.pets.contains_key(&expected));
        engine
            .dispatch(Command::CreatePet {
                species: SpeciesId::new("aquafin").unwrap(),
                name: PetName::new("Bubbles").unwrap(),
            })
            .unwrap();
        assert_eq!(engine.state.simulation.pets.len(), BASE_ADOPTED_PETS);

        let slot = SaveSlotId::derive(seed, Tick::ZERO, 99);
        let events = engine.dispatch(Command::ResetGame { slot }).unwrap();
        assert_eq!(events, vec![DomainEvent::WorldReset]);
        assert!(engine.state.simulation.pets.is_empty());
        assert!(engine.state.inventory.is_empty());
        assert_eq!(engine.state.simulation.tick, Tick::ZERO);
    }

    #[test]
    fn adoption_slots_have_escalating_prices_and_failed_purchases_are_atomic() {
        let seed = WorldSeed(17);
        let mut state = GameState::new(seed, UnixSeconds::EPOCH);
        state.wallet = domain::economy::Wallet::with_balance(Currency(350));
        let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let content = content::ContentLoader::load(&assets).unwrap();
        let config = simulation::SimConfig::from_ron(
            &std::fs::read_to_string(assets.join("config/simulation.ron")).unwrap(),
        )
        .unwrap();
        let mut engine = GameEngine::new(state, content, config, NoSaves);

        engine.dispatch(Command::UnlockAdoptionSlot).unwrap();
        assert_eq!(engine.state.adoption_slot_upgrades, 1);
        assert_eq!(engine.state.wallet.balance(), Currency(250));
        assert_eq!(adopted_pet_capacity(1), 3);

        engine.dispatch(Command::UnlockAdoptionSlot).unwrap();
        assert_eq!(engine.state.adoption_slot_upgrades, 2);
        assert_eq!(engine.state.wallet.balance(), Currency(50));
        assert_eq!(next_adoption_slot_cost(2), Some(Currency(400)));

        assert!(engine.dispatch(Command::UnlockAdoptionSlot).is_err());
        assert_eq!(engine.state.adoption_slot_upgrades, 2);
        assert_eq!(engine.state.wallet.balance(), Currency(50));
    }

    #[test]
    fn feeding_is_validated_before_inventory_is_consumed() {
        let seed = WorldSeed(1);
        let mut state = GameState::new(seed, UnixSeconds(0));
        let pet_id = PetId::derive(seed, Tick::ZERO, 1);
        let mut pet = Pet::hatch(
            pet_id,
            SpeciesId::new("emberling").unwrap(),
            GeneticProfile::new(),
            Personality::neutral(),
            Parentage::Founder,
            Ancestry::founder(),
            Tick::ZERO,
        );
        pet.set_need(NeedKind::Hunger, NeedValue::EMPTY, Tick::ZERO);
        state.simulation.insert_pet(pet);
        let item = ItemId::new("apple_slice").unwrap();
        state.inventory.add(item.clone(), 1).unwrap();
        let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let content = content::ContentLoader::load(&assets).unwrap();
        let config = simulation::SimConfig::from_ron(
            &std::fs::read_to_string(assets.join("config/simulation.ron")).unwrap(),
        )
        .unwrap();
        let mut engine = GameEngine::new(state, content, config, NoSaves);
        let events = engine
            .dispatch(Command::FeedPet {
                pet: pet_id,
                item: item.clone(),
            })
            .unwrap();
        assert_eq!(engine.state.inventory.count(&item), 0);
        assert!(!events.is_empty());
        assert!(engine.state.simulation.pet(pet_id).unwrap().needs.hunger > NeedValue::EMPTY);
    }

    #[test]
    fn breeding_then_hatching_updates_authoritative_state() {
        let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let content = content::ContentLoader::load(&assets).unwrap();
        let config = simulation::SimConfig::from_ron(
            &std::fs::read_to_string(assets.join("config/simulation.ron")).unwrap(),
        )
        .unwrap();

        let mut successful = None;
        for raw_seed in 1..=64 {
            let seed = WorldSeed(raw_seed);
            let mut state = GameState::new(seed, UnixSeconds(0));
            state.simulation.tick = Tick(20_000);
            state
                .clock
                .advance(domain::time::GameDuration::from_ticks(20_000));
            let parents = [
                PetId::derive(seed, Tick::ZERO, 1),
                PetId::derive(seed, Tick::ZERO, 2),
            ];
            for parent in parents {
                let mut pet = Pet::hatch(
                    parent,
                    SpeciesId::new("emberling").unwrap(),
                    GeneticProfile::new(),
                    Personality::neutral(),
                    Parentage::Founder,
                    Ancestry::founder(),
                    Tick::ZERO,
                );
                pet.stage = domain::pet::LifeStage::Adult;
                state.simulation.insert_pet(pet);
            }
            let mut engine = GameEngine::new(state, content.clone(), config.clone(), NoSaves);
            if engine
                .dispatch(Command::Breed {
                    first: parents[0],
                    second: parents[1],
                })
                .is_ok()
            {
                successful = Some(engine);
                break;
            }
        }
        let mut engine = successful.expect("at least one deterministic seed should breed");
        let egg_id = *engine.state.eggs.keys().next().unwrap();
        let parent = engine.state.eggs[&egg_id].parents[0];
        assert!(matches!(
            engine.dispatch(Command::ReleasePet { pet: parent }),
            Err(AppError::Domain(detail)) if detail == "a pet with an unhatched egg cannot be released"
        ));
        let incubation = domain::time::GameDuration::from_days(2);
        engine.state.simulation.tick = engine.state.simulation.tick.saturating_add(incubation);
        engine.state.clock.advance(incubation);
        let pet_count = engine.state.simulation.pets.len();

        let events = engine.dispatch(Command::HatchEgg { egg: egg_id }).unwrap();

        assert_eq!(events.len(), 1);
        assert!(engine.state.eggs.is_empty());
        assert_eq!(engine.state.simulation.pets.len(), pet_count + 1);
        engine.state.validate().unwrap();
    }

    #[test]
    fn resolving_a_battle_awards_once_and_closes_the_session() {
        let seed = WorldSeed(55);
        let mut state = GameState::new(seed, UnixSeconds(0));
        let pets = [
            PetId::derive(seed, Tick::ZERO, 1),
            PetId::derive(seed, Tick::ZERO, 2),
        ];
        for pet_id in pets {
            let mut pet = Pet::hatch(
                pet_id,
                SpeciesId::new("emberling").unwrap(),
                GeneticProfile::new(),
                Personality::neutral(),
                Parentage::Founder,
                Ancestry::founder(),
                Tick::ZERO,
            );
            pet.stage = domain::pet::LifeStage::Adult;
            state.simulation.insert_pet(pet);
        }
        let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let content = content::ContentLoader::load(&assets).unwrap();
        let config = simulation::SimConfig::from_ron(
            &std::fs::read_to_string(assets.join("config/simulation.ron")).unwrap(),
        )
        .unwrap();
        let mut engine = GameEngine::new(state, content, config, NoSaves);
        engine
            .dispatch(Command::StartBattle {
                first: pets[0],
                second: pets[1],
            })
            .unwrap();
        assert!(matches!(
            engine.dispatch(Command::ReleasePet { pet: pets[0] }),
            Err(AppError::Domain(detail)) if detail == "a pet in an active battle cannot be released"
        ));
        let battle = *engine.state.active_battles.keys().next().unwrap();
        let move_id = domain::identifiers::MoveId::new("flame_arc").unwrap();
        let actions = (0..4)
            .map(|_| {
                [
                    BattleAction {
                        actor: pets[0],
                        move_id: move_id.clone(),
                    },
                    BattleAction {
                        actor: pets[1],
                        move_id: move_id.clone(),
                    },
                ]
            })
            .collect();

        let events = engine
            .dispatch(Command::ResolveBattle { battle, actions })
            .unwrap();

        assert!(engine.state.active_battles.is_empty());
        assert_eq!(engine.state.wallet.balance(), Currency(50));
        let winner = events.iter().find_map(|event| match event {
            DomainEvent::Combat(CombatEvent::BattleWon {
                winner,
                levels_gained,
                ..
            }) => Some((*winner, *levels_gained)),
            _ => None,
        });
        let (winner, levels_gained) = winner.expect("battle won event");
        assert_eq!(levels_gained, 1);
        assert_eq!(
            engine
                .state
                .simulation
                .pet(winner)
                .unwrap()
                .progression
                .level(),
            domain::pet::Level::new(2)
        );
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::Combat(CombatEvent::BattleWon { battle: id, .. }) if *id == battle
        )));
        assert!(
            engine
                .dispatch(Command::ResolveBattle {
                    battle,
                    actions: Vec::new()
                })
                .is_err()
        );
        assert_eq!(engine.state.wallet.balance(), Currency(50));
    }
}
