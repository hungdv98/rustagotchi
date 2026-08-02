//! On-disk content shapes. These deliberately use authored strings and decimals.

use crate::decimal::Decimal;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpeciesDefinition {
    pub id: String,
    pub name_key: String,
    pub element: ElementDefinition,
    #[serde(default)]
    pub base: bool,
    pub move_pool: BTreeMap<StageDefinition, Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvolutionDefinition {
    pub from: String,
    pub to: String,
    pub minimum_level: u16,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemDefinition {
    pub id: String,
    pub name_key: String,
    pub category: ItemCategoryDefinition,
    pub potency_percent: Decimal,
    pub price: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MoveDefinition {
    pub id: String,
    pub name_key: String,
    pub element: ElementDefinition,
    pub power: u16,
    pub accuracy_percent: Decimal,
    pub energy_cost: u16,
    #[serde(default)]
    pub status: Option<StatusDefinition>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatusDefinition {
    pub kind: StatusKindDefinition,
    pub chance_percent: Decimal,
    pub duration_turns: u8,
    pub potency: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusKindDefinition {
    Burning,
    Poisoned,
    Stunned,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IllnessDefinition {
    pub id: String,
    pub name_key: String,
    pub severity_per_tick: Decimal,
    pub medicine: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompatibilityDefinition {
    pub first: String,
    pub second: String,
    pub chance_percent: Decimal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ElementDefinition {
    Neutral,
    Ember,
    Aqua,
    Storm,
    Terra,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemCategoryDefinition {
    Food,
    Medicine,
    Toy,
    Equipment,
    Material,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageDefinition {
    Hatchling,
    Child,
    Teen,
    Adult,
    Elder,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefinitionList<T> {
    pub definitions: Vec<T>,
}

pub type Translations = BTreeMap<String, String>;
