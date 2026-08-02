//! Read-only application queries returning presentation-safe DTOs.

use domain::pet::NeedKind;

use crate::GameState;
use crate::dto::{
    BattleDto, BattleMoveDto, BattlePetDto, BattleSetupDto, EggDto, GameSummaryDto,
    InventoryItemDto, NeedDto, PetDto,
};

pub trait Query {
    type Output;

    fn execute(self, state: &GameState) -> Self::Output;
}

pub trait ContentQuery {
    type Output;

    fn execute(self, state: &GameState, content: &content::ContentRegistry) -> Self::Output;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct GameSummaryQuery;

impl Query for GameSummaryQuery {
    type Output = GameSummaryDto;

    fn execute(self, state: &GameState) -> Self::Output {
        let pets = state
            .simulation
            .pets
            .values()
            .map(|pet| PetDto {
                id: pet.id.to_string(),
                name: pet.name.to_string(),
                species: pet.species.to_string(),
                life_stage: pet.stage.name().to_owned(),
                mood: pet.mood().name().to_owned(),
                activity: pet.activity.current().name().to_owned(),
                alive: pet.is_alive(),
                age_ticks: pet.progression.age(state.simulation.tick).as_ticks(),
                level: pet.progression.level().get(),
                experience: pet.progression.experience().0,
                experience_for_next_level: domain::pet::PetProgression::required_for_next(
                    pet.progression.level(),
                ),
                experience_progress_milli: pet.progression.progress_to_next().0,
                needs: NeedKind::ALL
                    .into_iter()
                    .map(|kind| NeedDto {
                        kind: kind.name().to_owned(),
                        value_milli: pet.needs.get(kind).get().0,
                    })
                    .collect(),
            })
            .collect();
        let inventory = state
            .inventory
            .iter()
            .map(|(item, quantity)| InventoryItemDto {
                item_id: item.to_string(),
                quantity,
            })
            .collect();
        let eggs = state
            .eggs
            .values()
            .map(|egg| EggDto {
                id: egg.id.to_string(),
                species: egg.species.to_string(),
                parent_ids: [egg.parents[0].to_string(), egg.parents[1].to_string()],
                remaining_ticks: egg.incubation.remaining(state.simulation.tick).as_ticks(),
                ready: egg.is_ready(state.simulation.tick),
            })
            .collect();
        let active_battles = state
            .active_battles
            .values()
            .map(|battle| BattleDto {
                id: battle.id.to_string(),
                combatant_ids: [
                    battle.combatants[0].pet.to_string(),
                    battle.combatants[1].pet.to_string(),
                ],
                health: [battle.combatants[0].health, battle.combatants[1].health],
                energy: [battle.combatants[0].energy, battle.combatants[1].energy],
                turn: battle.turn,
            })
            .collect();

        GameSummaryDto {
            tick: state.simulation.tick.0,
            balance: state.wallet.balance().0,
            clamped_offline_ticks: state.simulation.clamped_offline_ticks,
            adopted_pets: state
                .simulation
                .pets
                .values()
                .filter(|pet| pet.parentage.is_founder())
                .count(),
            max_adopted_pets: crate::adopted_pet_capacity(state.adoption_slot_upgrades),
            maximum_adopted_pets: crate::MAX_ADOPTED_PETS,
            next_adoption_slot_cost: crate::next_adoption_slot_cost(state.adoption_slot_upgrades)
                .map(|cost| cost.0),
            pets,
            eggs,
            active_battles,
            inventory,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BattleSetupQuery;

impl ContentQuery for BattleSetupQuery {
    type Output = BattleSetupDto;

    fn execute(self, state: &GameState, content: &content::ContentRegistry) -> Self::Output {
        let pets = state
            .simulation
            .pets
            .values()
            .filter(|pet| pet.is_alive())
            .filter_map(|pet| {
                let species = content.get_species(&pet.species)?;
                let moves = species
                    .move_pool
                    .get(&pet.stage)?
                    .iter()
                    .filter_map(|move_id| content.get_move(move_id))
                    .map(|definition| BattleMoveDto {
                        id: definition.id.to_string(),
                        power: definition.power,
                        accuracy_milli: definition.accuracy.0,
                        energy_cost: definition.energy_cost,
                    })
                    .collect();
                Some(BattlePetDto {
                    id: pet.id.to_string(),
                    name: pet.name.to_string(),
                    species: pet.species.to_string(),
                    moves,
                })
            })
            .collect();
        BattleSetupDto { pets }
    }
}

#[cfg(test)]
mod tests {
    use domain::identifiers::ItemId;
    use domain::rng::WorldSeed;
    use domain::time::UnixSeconds;

    use super::*;

    #[test]
    fn summary_contains_only_stable_view_data() {
        let mut state = GameState::new(WorldSeed(7), UnixSeconds(100));
        state
            .inventory
            .add(ItemId::new("apple_slice").unwrap(), 2)
            .unwrap();

        let summary = GameSummaryQuery.execute(&state);

        assert_eq!(summary.tick, 0);
        assert_eq!(summary.inventory[0].item_id, "apple_slice");
        assert_eq!(summary.inventory[0].quantity, 2);
        assert_eq!(summary.max_adopted_pets, crate::BASE_ADOPTED_PETS);
        assert_eq!(summary.maximum_adopted_pets, crate::MAX_ADOPTED_PETS);
        assert_eq!(summary.next_adoption_slot_cost, Some(100));
    }

    #[test]
    fn summary_exposes_activity_as_stable_presentation_text() {
        use domain::genetics::{Ancestry, GeneticProfile, Parentage};
        use domain::identifiers::{PetId, SpeciesId};
        use domain::pet::{Activity, Personality, Pet};
        use domain::time::Tick;

        let seed = WorldSeed(12);
        let mut state = GameState::new(seed, UnixSeconds::EPOCH);
        let mut pet = Pet::hatch(
            PetId::derive(seed, Tick::ZERO, 1),
            SpeciesId::new("emberling").unwrap(),
            GeneticProfile::new(),
            Personality::neutral(),
            Parentage::Founder,
            Ancestry::founder(),
            Tick::ZERO,
        );
        pet.activity.begin_open_ended(Activity::Playing, Tick::ZERO);
        state.simulation.insert_pet(pet);

        let summary = GameSummaryQuery.execute(&state);

        assert_eq!(summary.pets[0].activity, "playing");
        assert_eq!(summary.pets[0].name, "Companion");
    }

    #[test]
    fn battle_setup_uses_the_authored_move_pool_for_the_current_stage() {
        use domain::genetics::{Ancestry, GeneticProfile, Parentage};
        use domain::identifiers::{PetId, SpeciesId};
        use domain::pet::{Personality, Pet};
        use domain::time::Tick;

        let seed = WorldSeed(8);
        let mut state = GameState::new(seed, UnixSeconds(0));
        state.simulation.insert_pet(Pet::hatch(
            PetId::derive(seed, Tick::ZERO, 1),
            SpeciesId::new("emberling").unwrap(),
            GeneticProfile::new(),
            Personality::neutral(),
            Parentage::Founder,
            Ancestry::founder(),
            Tick::ZERO,
        ));
        let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let content = content::ContentLoader::load(assets).unwrap();

        let setup = BattleSetupQuery.execute(&state, &content);

        assert_eq!(setup.pets.len(), 1);
        assert_eq!(setup.pets[0].moves[0].id, "tackle");
        assert_eq!(setup.pets[0].moves[1].id, "warm_nuzzle");
    }
}
