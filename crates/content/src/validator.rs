use std::collections::{BTreeMap, BTreeSet};

use crate::definitions::StageDefinition;
use crate::error::{ContentIssue, SourceLocation, ValidationRule};
use crate::loader::RawContent;

pub(crate) fn validate(raw: &RawContent) -> Vec<ContentIssue> {
    let mut issues = Vec::new();
    let species: BTreeSet<_> = raw.species.iter().map(|v| v.value.id.as_str()).collect();
    let moves: BTreeSet<_> = raw.moves.iter().map(|v| v.value.id.as_str()).collect();
    let items: BTreeSet<_> = raw.items.iter().map(|v| v.value.id.as_str()).collect();
    validate_ids(raw, &mut issues);
    validate_species(raw, &species, &moves, &mut issues);
    validate_evolutions(raw, &species, &mut issues);
    validate_items(raw, &mut issues);
    validate_moves(raw, &mut issues);
    validate_illnesses(raw, &items, &mut issues);
    validate_compatibility(raw, &species, &mut issues);
    validate_translations(raw, &mut issues);
    issues.sort_by(|a, b| {
        a.rule
            .cmp(&b.rule)
            .then_with(|| a.location.file.cmp(&b.location.file))
            .then_with(|| a.message.cmp(&b.message))
    });
    issues
}

fn validate_ids(raw: &RawContent, issues: &mut Vec<ContentIssue>) {
    let mut seen: BTreeMap<&str, (&str, &std::path::Path)> = BTreeMap::new();
    let entries = raw
        .species
        .iter()
        .map(|v| ("species", v.value.id.as_str(), v.file.as_path()))
        .chain(
            raw.items
                .iter()
                .map(|v| ("item", v.value.id.as_str(), v.file.as_path())),
        )
        .chain(
            raw.moves
                .iter()
                .map(|v| ("move", v.value.id.as_str(), v.file.as_path())),
        )
        .chain(
            raw.illnesses
                .iter()
                .map(|v| ("illness", v.value.id.as_str(), v.file.as_path())),
        );
    for (kind, id, file) in entries {
        if domain::identifiers::ContentKey::new("content id", id).is_err() {
            push(
                issues,
                ValidationRule::ValueOutOfRange,
                file,
                format!("{kind} id {id:?} must be snake_case"),
            );
        }
        if let Some((previous_kind, previous_file)) = seen.insert(id, (kind, file)) {
            push(
                issues,
                ValidationRule::DuplicateId,
                file,
                format!(
                    "{kind} `{id}` duplicates {previous_kind} from {}",
                    previous_file.display()
                ),
            );
        }
    }
}

fn validate_species(
    raw: &RawContent,
    species: &BTreeSet<&str>,
    moves: &BTreeSet<&str>,
    issues: &mut Vec<ContentIssue>,
) {
    let stages = [
        StageDefinition::Hatchling,
        StageDefinition::Child,
        StageDefinition::Teen,
        StageDefinition::Adult,
        StageDefinition::Elder,
    ];
    for entry in &raw.species {
        for stage in stages {
            match entry.value.move_pool.get(&stage) {
                Some(pool) if !pool.is_empty() => {
                    for move_id in pool {
                        if !moves.contains(move_id.as_str()) {
                            push(
                                issues,
                                ValidationRule::DanglingReference,
                                &entry.file,
                                format!(
                                    "species `{}` references unknown move `{move_id}`",
                                    entry.value.id
                                ),
                            );
                        }
                    }
                }
                _ => push(
                    issues,
                    ValidationRule::EmptyMovePool,
                    &entry.file,
                    format!("species `{}` has no move for {stage:?}", entry.value.id),
                ),
            }
        }
    }
    if species.is_empty() {
        push(
            issues,
            ValidationRule::UnreachableSpecies,
            &raw.manifest.en_us,
            "content pack defines no species".into(),
        );
    }
}

fn validate_evolutions(raw: &RawContent, species: &BTreeSet<&str>, issues: &mut Vec<ContentIssue>) {
    let mut edges: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for edge in &raw.evolutions {
        for id in [&edge.value.from, &edge.value.to] {
            if !species.contains(id.as_str()) {
                push(
                    issues,
                    ValidationRule::DanglingReference,
                    &edge.file,
                    format!("evolution references unknown species `{id}`"),
                );
            }
        }
        if edge.value.minimum_level == 0 || edge.value.minimum_level > 100 {
            push(
                issues,
                ValidationRule::ValueOutOfRange,
                &edge.file,
                format!(
                    "evolution {} -> {} level must be 1..=100",
                    edge.value.from, edge.value.to
                ),
            );
        }
        edges
            .entry(&edge.value.from)
            .or_default()
            .push(&edge.value.to);
    }
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    for id in species {
        if find_cycle(id, &edges, &mut visiting, &mut visited) {
            push(
                issues,
                ValidationRule::EvolutionCycle,
                &raw.manifest.en_us,
                format!("evolution cycle includes `{id}`"),
            );
            break;
        }
    }
    let mut reachable: BTreeSet<&str> = raw
        .species
        .iter()
        .filter(|s| s.value.base)
        .map(|s| s.value.id.as_str())
        .collect();
    let mut frontier: Vec<&str> = reachable.iter().copied().collect();
    while let Some(id) = frontier.pop() {
        for next in edges.get(id).into_iter().flatten() {
            if reachable.insert(next) {
                frontier.push(next);
            }
        }
    }
    for id in species.difference(&reachable) {
        push(
            issues,
            ValidationRule::UnreachableSpecies,
            &raw.manifest.en_us,
            format!("species `{id}` is unreachable from a base species"),
        );
    }
}

fn find_cycle<'a>(
    id: &'a str,
    edges: &BTreeMap<&'a str, Vec<&'a str>>,
    visiting: &mut BTreeSet<&'a str>,
    visited: &mut BTreeSet<&'a str>,
) -> bool {
    if visited.contains(id) {
        return false;
    }
    if !visiting.insert(id) {
        return true;
    }
    if edges
        .get(id)
        .into_iter()
        .flatten()
        .any(|next| find_cycle(next, edges, visiting, visited))
    {
        return true;
    }
    visiting.remove(id);
    visited.insert(id);
    false
}

fn validate_items(raw: &RawContent, issues: &mut Vec<ContentIssue>) {
    for v in &raw.items {
        if v.value.potency_percent.is_negative()
            || v.value.potency_percent.fixed() > domain::units::Fixed::from_int(100)
        {
            push(
                issues,
                ValidationRule::ValueOutOfRange,
                &v.file,
                format!("item `{}` potency must be 0..=100", v.value.id),
            );
        }
        if v.value.price > 1_000_000 {
            push(
                issues,
                ValidationRule::ValueOutOfRange,
                &v.file,
                format!("item `{}` price exceeds 1000000", v.value.id),
            );
        }
    }
}
fn validate_moves(raw: &RawContent, issues: &mut Vec<ContentIssue>) {
    for v in &raw.moves {
        if !(1..=500).contains(&v.value.power) {
            push(
                issues,
                ValidationRule::ValueOutOfRange,
                &v.file,
                format!("move `{}` power must be 1..=500", v.value.id),
            );
        }
        if v.value.accuracy_percent.is_negative()
            || v.value.accuracy_percent.fixed() > domain::units::Fixed::from_int(100)
        {
            push(
                issues,
                ValidationRule::ValueOutOfRange,
                &v.file,
                format!("move `{}` accuracy must be 0..=100", v.value.id),
            );
        }
        if v.value.energy_cost > 100 {
            push(
                issues,
                ValidationRule::ValueOutOfRange,
                &v.file,
                format!("move `{}` energy cost must be <=100", v.value.id),
            );
        }
        if let Some(status) = &v.value.status
            && (status.duration_turns == 0
                || status.duration_turns > 20
                || status.potency > 1_000
                || status.chance_percent.is_negative()
                || status.chance_percent.fixed() > domain::units::Fixed::from_int(100))
        {
            push(
                issues,
                ValidationRule::ValueOutOfRange,
                &v.file,
                format!("move `{}` has invalid status parameters", v.value.id),
            );
        }
    }
}
fn validate_illnesses(raw: &RawContent, items: &BTreeSet<&str>, issues: &mut Vec<ContentIssue>) {
    for v in &raw.illnesses {
        if !items.contains(v.value.medicine.as_str()) {
            push(
                issues,
                ValidationRule::DanglingReference,
                &v.file,
                format!(
                    "illness `{}` references unknown medicine `{}`",
                    v.value.id, v.value.medicine
                ),
            );
        }
        if let Some(item) = raw
            .items
            .iter()
            .find(|item| item.value.id == v.value.medicine)
            && !matches!(
                item.value.category,
                crate::definitions::ItemCategoryDefinition::Medicine
            )
        {
            push(
                issues,
                ValidationRule::DanglingReference,
                &v.file,
                format!(
                    "illness `{}` cure `{}` is not a medicine",
                    v.value.id, v.value.medicine
                ),
            );
        }
        if v.value.severity_per_tick.is_negative()
            || v.value.severity_per_tick.fixed() > domain::units::Fixed::ONE
        {
            push(
                issues,
                ValidationRule::ValueOutOfRange,
                &v.file,
                format!("illness `{}` severity_per_tick must be 0..=1", v.value.id),
            );
        }
    }
}

fn validate_compatibility(
    raw: &RawContent,
    species: &BTreeSet<&str>,
    issues: &mut Vec<ContentIssue>,
) {
    let mut matrix = BTreeMap::new();
    for v in &raw.compatibility {
        for id in [&v.value.first, &v.value.second] {
            if !species.contains(id.as_str()) {
                push(
                    issues,
                    ValidationRule::DanglingReference,
                    &v.file,
                    format!("compatibility references unknown species `{id}`"),
                );
            }
        }
        if v.value.chance_percent.is_negative()
            || v.value.chance_percent.fixed() > domain::units::Fixed::from_int(100)
        {
            push(
                issues,
                ValidationRule::ValueOutOfRange,
                &v.file,
                "compatibility chance must be 0..=100".into(),
            );
        }
        if matrix
            .insert(
                (v.value.first.as_str(), v.value.second.as_str()),
                v.value.chance_percent,
            )
            .is_some()
        {
            push(
                issues,
                ValidationRule::DuplicateId,
                &v.file,
                format!(
                    "compatibility `{}` -> `{}` is defined twice",
                    v.value.first, v.value.second
                ),
            );
        }
    }
    for first in species {
        for second in species {
            match (matrix.get(&(first, second)), matrix.get(&(second, first))) {
                (None, _) => push(
                    issues,
                    ValidationRule::IncompleteCompatibility,
                    &raw.manifest.en_us,
                    format!("missing compatibility `{first}` -> `{second}`"),
                ),
                (Some(a), Some(b)) if a != b => push(
                    issues,
                    ValidationRule::AsymmetricCompatibility,
                    &raw.manifest.en_us,
                    format!("compatibility `{first}`/`{second}` is asymmetric"),
                ),
                _ => {}
            }
        }
    }
}

fn validate_translations(raw: &RawContent, issues: &mut Vec<ContentIssue>) {
    let entries = raw
        .species
        .iter()
        .map(|v| {
            (
                v.value.id.as_str(),
                v.value.name_key.as_str(),
                v.file.as_path(),
            )
        })
        .chain(raw.items.iter().map(|v| {
            (
                v.value.id.as_str(),
                v.value.name_key.as_str(),
                v.file.as_path(),
            )
        }))
        .chain(raw.moves.iter().map(|v| {
            (
                v.value.id.as_str(),
                v.value.name_key.as_str(),
                v.file.as_path(),
            )
        }))
        .chain(raw.illnesses.iter().map(|v| {
            (
                v.value.id.as_str(),
                v.value.name_key.as_str(),
                v.file.as_path(),
            )
        }));
    for (id, key, file) in entries {
        if raw.translations.get(key).is_none_or(String::is_empty) {
            push(
                issues,
                ValidationRule::MissingTranslation,
                file,
                format!("`{id}` requires non-empty en-US key `{key}`"),
            );
        }
    }
}

fn push(
    issues: &mut Vec<ContentIssue>,
    rule: ValidationRule,
    file: &std::path::Path,
    message: String,
) {
    issues.push(ContentIssue {
        rule,
        location: SourceLocation::file(file),
        message,
    });
}
