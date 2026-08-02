//! Immutable, validated runtime content.

use std::collections::HashMap;

use domain::combat::{Element, StatusKind};
use domain::identifiers::{IllnessId, ItemId, MoveId, SpeciesId};
use domain::inventory::ItemCategory;
use domain::pet::LifeStage;
use domain::units::{Fixed, Milli};

use crate::definitions::{ElementDefinition, ItemCategoryDefinition, StageDefinition};
use crate::loader::RawContent;

#[derive(Debug, Clone)]
pub struct SpeciesDef {
    pub id: SpeciesId,
    pub name_key: String,
    pub element: Element,
    pub base: bool,
    pub move_pool: HashMap<LifeStage, Vec<MoveId>>,
}
#[derive(Debug, Clone)]
pub struct EvolutionDef {
    pub from: SpeciesId,
    pub to: SpeciesId,
    pub minimum_level: u16,
}
#[derive(Debug, Clone)]
pub struct ItemDef {
    pub id: ItemId,
    pub name_key: String,
    pub category: ItemCategory,
    pub potency: Milli,
    pub price: u32,
}
#[derive(Debug, Clone)]
pub struct MoveDef {
    pub id: MoveId,
    pub name_key: String,
    pub element: Element,
    pub power: u16,
    pub accuracy: Milli,
    pub energy_cost: u16,
    pub status: Option<MoveStatusDef>,
}
#[derive(Debug, Clone, Copy)]
pub struct MoveStatusDef {
    pub kind: StatusKind,
    pub chance: Milli,
    pub duration_turns: u8,
    pub potency: u16,
}
#[derive(Debug, Clone)]
pub struct IllnessDef {
    pub id: IllnessId,
    pub name_key: String,
    pub severity_per_tick: Fixed,
    pub medicine: ItemId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContentCounts {
    pub species: usize,
    pub evolutions: usize,
    pub items: usize,
    pub moves: usize,
    pub illnesses: usize,
    pub compatibility_entries: usize,
    pub translations: usize,
}

#[derive(Debug, Clone)]
pub struct ContentRegistry {
    pack_id: String,
    species: Vec<SpeciesDef>,
    species_index: HashMap<SpeciesId, usize>,
    evolutions: Vec<EvolutionDef>,
    items: Vec<ItemDef>,
    item_index: HashMap<ItemId, usize>,
    moves: Vec<MoveDef>,
    move_index: HashMap<MoveId, usize>,
    illnesses: Vec<IllnessDef>,
    illness_index: HashMap<IllnessId, usize>,
    compatibility: HashMap<(SpeciesId, SpeciesId), Milli>,
    translations: HashMap<String, String>,
}

impl ContentRegistry {
    pub(crate) fn from_validated(raw: RawContent) -> Self {
        let species: Vec<_> = raw
            .species
            .into_iter()
            .map(|v| SpeciesDef {
                id: SpeciesId::new(&v.value.id).expect("validated"),
                name_key: v.value.name_key,
                element: element(v.value.element),
                base: v.value.base,
                move_pool: v
                    .value
                    .move_pool
                    .into_iter()
                    .map(|(stage, ids)| {
                        (
                            life_stage(stage),
                            ids.into_iter()
                                .map(|id| MoveId::new(&id).expect("validated reference"))
                                .collect(),
                        )
                    })
                    .collect(),
            })
            .collect();
        let items: Vec<_> = raw
            .items
            .into_iter()
            .map(|v| ItemDef {
                id: ItemId::new(&v.value.id).expect("validated"),
                name_key: v.value.name_key,
                category: category(v.value.category),
                potency: v.value.potency_percent.as_percent(),
                price: v.value.price,
            })
            .collect();
        let moves: Vec<_> = raw
            .moves
            .into_iter()
            .map(|v| MoveDef {
                id: MoveId::new(&v.value.id).expect("validated"),
                name_key: v.value.name_key,
                element: element(v.value.element),
                power: v.value.power,
                accuracy: v.value.accuracy_percent.as_percent(),
                energy_cost: v.value.energy_cost,
                status: v.value.status.map(|status| MoveStatusDef {
                    kind: match status.kind {
                        crate::definitions::StatusKindDefinition::Burning => StatusKind::Burning,
                        crate::definitions::StatusKindDefinition::Poisoned => StatusKind::Poisoned,
                        crate::definitions::StatusKindDefinition::Stunned => StatusKind::Stunned,
                    },
                    chance: status.chance_percent.as_percent(),
                    duration_turns: status.duration_turns,
                    potency: status.potency,
                }),
            })
            .collect();
        let illnesses: Vec<_> = raw
            .illnesses
            .into_iter()
            .map(|v| IllnessDef {
                id: IllnessId::new(&v.value.id).expect("validated"),
                name_key: v.value.name_key,
                severity_per_tick: v.value.severity_per_tick.fixed(),
                medicine: ItemId::new(&v.value.medicine).expect("validated reference"),
            })
            .collect();
        let evolutions = raw
            .evolutions
            .into_iter()
            .map(|v| EvolutionDef {
                from: SpeciesId::new(&v.value.from).expect("validated reference"),
                to: SpeciesId::new(&v.value.to).expect("validated reference"),
                minimum_level: v.value.minimum_level,
            })
            .collect();
        let compatibility = raw
            .compatibility
            .into_iter()
            .map(|v| {
                (
                    (
                        SpeciesId::new(&v.value.first).expect("validated reference"),
                        SpeciesId::new(&v.value.second).expect("validated reference"),
                    ),
                    v.value.chance_percent.as_percent(),
                )
            })
            .collect();
        let species_index = index(&species, |v| v.id.clone());
        let item_index = index(&items, |v| v.id.clone());
        let move_index = index(&moves, |v| v.id.clone());
        let illness_index = index(&illnesses, |v| v.id.clone());
        Self {
            pack_id: raw.manifest.pack_id,
            species,
            species_index,
            evolutions,
            items,
            item_index,
            moves,
            move_index,
            illnesses,
            illness_index,
            compatibility,
            translations: raw.translations.into_iter().collect(),
        }
    }

    #[must_use]
    pub fn pack_id(&self) -> &str {
        &self.pack_id
    }
    #[must_use]
    pub fn get_species(&self, id: &SpeciesId) -> Option<&SpeciesDef> {
        self.species_index.get(id).map(|i| &self.species[*i])
    }
    #[must_use]
    pub fn get_item(&self, id: &ItemId) -> Option<&ItemDef> {
        self.item_index.get(id).map(|i| &self.items[*i])
    }
    #[must_use]
    pub fn get_move(&self, id: &MoveId) -> Option<&MoveDef> {
        self.move_index.get(id).map(|i| &self.moves[*i])
    }
    #[must_use]
    pub fn get_illness(&self, id: &IllnessId) -> Option<&IllnessDef> {
        self.illness_index.get(id).map(|i| &self.illnesses[*i])
    }
    #[must_use]
    pub fn compatibility(&self, first: &SpeciesId, second: &SpeciesId) -> Option<Milli> {
        self.compatibility
            .get(&(first.clone(), second.clone()))
            .copied()
    }
    pub fn species(&self) -> impl ExactSizeIterator<Item = &SpeciesDef> {
        self.species.iter()
    }
    pub fn evolutions(&self) -> impl ExactSizeIterator<Item = &EvolutionDef> {
        self.evolutions.iter()
    }
    pub fn illnesses(&self) -> impl ExactSizeIterator<Item = &IllnessDef> {
        self.illnesses.iter()
    }
    #[must_use]
    pub fn translate(&self, key: &str) -> Option<&str> {
        self.translations.get(key).map(String::as_str)
    }
    #[must_use]
    pub fn counts(&self) -> ContentCounts {
        ContentCounts {
            species: self.species.len(),
            evolutions: self.evolutions.len(),
            items: self.items.len(),
            moves: self.moves.len(),
            illnesses: self.illnesses.len(),
            compatibility_entries: self.compatibility.len(),
            translations: self.translations.len(),
        }
    }
}

fn index<T, K: std::hash::Hash + Eq>(values: &[T], key: impl Fn(&T) -> K) -> HashMap<K, usize> {
    values
        .iter()
        .enumerate()
        .map(|(i, value)| (key(value), i))
        .collect()
}
const fn element(value: ElementDefinition) -> Element {
    match value {
        ElementDefinition::Neutral => Element::Neutral,
        ElementDefinition::Ember => Element::Ember,
        ElementDefinition::Aqua => Element::Aqua,
        ElementDefinition::Storm => Element::Storm,
        ElementDefinition::Terra => Element::Terra,
    }
}
const fn category(value: ItemCategoryDefinition) -> ItemCategory {
    match value {
        ItemCategoryDefinition::Food => ItemCategory::Food,
        ItemCategoryDefinition::Medicine => ItemCategory::Medicine,
        ItemCategoryDefinition::Toy => ItemCategory::Toy,
        ItemCategoryDefinition::Equipment => ItemCategory::Equipment,
        ItemCategoryDefinition::Material => ItemCategory::Material,
    }
}
const fn life_stage(value: StageDefinition) -> LifeStage {
    match value {
        StageDefinition::Hatchling => LifeStage::Hatchling,
        StageDefinition::Child => LifeStage::Child,
        StageDefinition::Teen => LifeStage::Teen,
        StageDefinition::Adult => LifeStage::Adult,
        StageDefinition::Elder => LifeStage::Elder,
    }
}
