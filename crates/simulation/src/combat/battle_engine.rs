use content::ContentRegistry;
use domain::combat::{
    BattleAction, BattleResult, BattleState, CombatEvent, StatusEffect, StatusKind,
};
use domain::rng::{RngStream, WorldSeed, rng_for};
use domain::time::Tick;
use domain::units::Milli;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CombatError {
    #[error("battle requires exactly one action from each combatant per turn")]
    InvalidActionSet,
    #[error("actor is not in this battle")]
    UnknownActor,
    #[error("unknown move `{0}`")]
    UnknownMove(String),
    #[error("actor lacks energy for move `{0}`")]
    InsufficientEnergy(String),
    #[error("battle did not finish within the supplied actions")]
    Unfinished,
}

pub fn resolve_battle(
    seed: WorldSeed,
    mut state: BattleState,
    actions: &[[BattleAction; 2]],
    content: &ContentRegistry,
) -> Result<BattleResult, CombatError> {
    let mut events = Vec::new();
    for pair in actions {
        if pair[0].actor == pair[1].actor {
            return Err(CombatError::InvalidActionSet);
        }
        let mut ordered = [0usize, 1usize];
        let actor_indices = [
            actor_index(&state, pair[0].actor)?,
            actor_index(&state, pair[1].actor)?,
        ];
        if actor_indices[0] == actor_indices[1] {
            return Err(CombatError::InvalidActionSet);
        }
        let agility = [
            state.combatants[actor_indices[0]].stats.agility.get(),
            state.combatants[actor_indices[1]].stats.agility.get(),
        ];
        if agility[1] > agility[0]
            || (agility[0] == agility[1]
                && rng_for(
                    seed,
                    RngStream::CombatAi,
                    Tick(u64::from(state.turn)),
                    state.id.rng_key(),
                )
                .chance(1, 2))
        {
            ordered.swap(0, 1);
        }

        for action_position in ordered {
            let action = &pair[action_position];
            let attacker = actor_index(&state, action.actor)?;
            let defender = 1 - attacker;
            if state.combatants[attacker].is_defeated() {
                continue;
            }
            if consume_stun(&mut state.combatants[attacker], &mut events) {
                continue;
            }
            let definition = content
                .get_move(&action.move_id)
                .ok_or_else(|| CombatError::UnknownMove(action.move_id.to_string()))?;
            if state.combatants[attacker].energy < definition.energy_cost {
                return Err(CombatError::InsufficientEnergy(action.move_id.to_string()));
            }
            state.combatants[attacker].energy -= definition.energy_cost;
            let mut hit_rng = rng_for(
                seed,
                RngStream::CombatHit,
                Tick(u64::from(state.turn)),
                state.combatants[attacker].pet.rng_key(),
            );
            if !hit_rng.chance(definition.accuracy.0.clamp(0, 100_000) as u32, 100_000) {
                events.push(CombatEvent::MoveMissed {
                    actor: action.actor,
                    move_id: action.move_id.clone(),
                });
                continue;
            }
            let attack = u32::from(state.combatants[attacker].stats.strength.get());
            let defense = u32::from(state.combatants[defender].stats.resilience.get());
            let base = u32::from(definition.power)
                .saturating_mul(attack)
                .checked_div(defense.max(1))
                .unwrap_or(1)
                .max(1);
            let scaled = definition
                .element
                .effectiveness_against(state.combatants[defender].element)
                .apply(Milli(base.min(i32::MAX as u32) as i32))
                .0
                .max(1) as u32;
            state.combatants[defender].health =
                state.combatants[defender].health.saturating_sub(scaled);
            events.push(CombatEvent::DamageDealt {
                actor: action.actor,
                target: state.combatants[defender].pet,
                move_id: action.move_id.clone(),
                amount: scaled,
            });
            if let Some(status) = definition.status {
                let mut status_rng = rng_for(
                    seed,
                    RngStream::CombatStatus,
                    Tick(u64::from(state.turn)),
                    state.combatants[attacker].pet.rng_key(),
                );
                if status_rng.chance(status.chance.0.clamp(0, 100_000) as u32, 100_000) {
                    apply_status(
                        &mut state.combatants[defender],
                        StatusEffect {
                            kind: status.kind,
                            remaining_turns: status.duration_turns,
                            potency: status.potency,
                        },
                    );
                    events.push(CombatEvent::StatusApplied {
                        target: state.combatants[defender].pet,
                        status: status.kind,
                        turns: status.duration_turns,
                    });
                }
            }
            if state.combatants[defender].is_defeated() {
                events.push(CombatEvent::Defeated {
                    pet: state.combatants[defender].pet,
                });
                state.turn = state.turn.saturating_add(1);
                return Ok(BattleResult {
                    winner: state.combatants[attacker].pet,
                    loser: state.combatants[defender].pet,
                    turns: state.turn,
                    final_state: state,
                    events,
                });
            }
        }
        process_end_of_turn(&mut state, &mut events);
        state.turn = state.turn.saturating_add(1);
        if let Some(loser) = state
            .combatants
            .iter()
            .position(|combatant| combatant.is_defeated())
        {
            let winner = 1 - loser;
            events.push(CombatEvent::Defeated {
                pet: state.combatants[loser].pet,
            });
            return Ok(BattleResult {
                winner: state.combatants[winner].pet,
                loser: state.combatants[loser].pet,
                turns: state.turn,
                final_state: state,
                events,
            });
        }
    }
    Err(CombatError::Unfinished)
}

fn apply_status(combatant: &mut domain::combat::Combatant, status: StatusEffect) {
    if let Some(existing) = combatant
        .statuses
        .iter_mut()
        .find(|existing| existing.kind == status.kind)
    {
        existing.remaining_turns = existing.remaining_turns.max(status.remaining_turns);
        existing.potency = existing.potency.max(status.potency);
    } else if combatant.statuses.len() < 8 {
        combatant.statuses.push(status);
    }
}

fn consume_stun(combatant: &mut domain::combat::Combatant, events: &mut Vec<CombatEvent>) -> bool {
    let Some(index) = combatant
        .statuses
        .iter()
        .position(|status| status.kind == StatusKind::Stunned)
    else {
        return false;
    };
    events.push(CombatEvent::StatusPreventedAction {
        pet: combatant.pet,
        status: StatusKind::Stunned,
    });
    combatant.statuses[index].remaining_turns =
        combatant.statuses[index].remaining_turns.saturating_sub(1);
    if combatant.statuses[index].remaining_turns == 0 {
        combatant.statuses.remove(index);
        events.push(CombatEvent::StatusExpired {
            pet: combatant.pet,
            status: StatusKind::Stunned,
        });
    }
    true
}

fn process_end_of_turn(state: &mut BattleState, events: &mut Vec<CombatEvent>) {
    for combatant in &mut state.combatants {
        let mut index = 0;
        while index < combatant.statuses.len() {
            let status = combatant.statuses[index];
            if matches!(status.kind, StatusKind::Burning | StatusKind::Poisoned) {
                let damage = u32::from(status.potency);
                combatant.health = combatant.health.saturating_sub(damage);
                events.push(CombatEvent::StatusDamaged {
                    pet: combatant.pet,
                    status: status.kind,
                    amount: damage,
                });
                combatant.statuses[index].remaining_turns =
                    status.remaining_turns.saturating_sub(1);
                if combatant.statuses[index].remaining_turns == 0 {
                    combatant.statuses.remove(index);
                    events.push(CombatEvent::StatusExpired {
                        pet: combatant.pet,
                        status: status.kind,
                    });
                    continue;
                }
            }
            index += 1;
        }
    }
}

fn actor_index(
    state: &BattleState,
    actor: domain::identifiers::PetId,
) -> Result<usize, CombatError> {
    state
        .combatants
        .iter()
        .position(|combatant| combatant.pet == actor)
        .ok_or(CombatError::UnknownActor)
}

#[cfg(test)]
mod tests {
    use content::ContentLoader;
    use domain::combat::{BattleState, Combatant, Element};
    use domain::identifiers::{BattleId, MoveId, PetId};
    use domain::pet::Stats;

    use super::*;

    #[test]
    fn mirror_battle_is_deterministic_and_terminates() {
        let seed = WorldSeed(44);
        let first = PetId::derive(seed, Tick::ZERO, 1);
        let second = PetId::derive(seed, Tick::ZERO, 2);
        let state = BattleState {
            id: BattleId::derive(seed, Tick::ZERO, 3),
            combatants: [
                Combatant::from_stats(first, Element::Neutral, Stats::uniform(10)),
                Combatant::from_stats(second, Element::Neutral, Stats::uniform(10)),
            ],
            turn: 0,
        };
        let tackle = MoveId::new("tackle").unwrap();
        let actions = (0..10)
            .map(|_| {
                [
                    BattleAction {
                        actor: first,
                        move_id: tackle.clone(),
                    },
                    BattleAction {
                        actor: second,
                        move_id: tackle.clone(),
                    },
                ]
            })
            .collect::<Vec<_>>();
        let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let content = ContentLoader::load(assets).unwrap();

        let first_result = resolve_battle(seed, state.clone(), &actions, &content).unwrap();
        let second_result = resolve_battle(seed, state, &actions, &content).unwrap();
        assert_eq!(first_result, second_result);
        assert_ne!(first_result.winner, first_result.loser);
        assert!(first_result.turns <= 10);
    }

    #[test]
    fn statuses_apply_damage_expire_and_can_skip_an_action() {
        let seed = WorldSeed(3);
        let pet = PetId::derive(seed, Tick::ZERO, 1);
        let other = PetId::derive(seed, Tick::ZERO, 2);
        let mut state = BattleState {
            id: BattleId::derive(seed, Tick::ZERO, 3),
            combatants: [
                Combatant::from_stats(pet, Element::Ember, Stats::uniform(10)),
                Combatant::from_stats(other, Element::Ember, Stats::uniform(10)),
            ],
            turn: 0,
        };
        state.combatants[0].statuses.push(StatusEffect {
            kind: StatusKind::Burning,
            remaining_turns: 1,
            potency: 7,
        });
        state.combatants[1].statuses.push(StatusEffect {
            kind: StatusKind::Stunned,
            remaining_turns: 1,
            potency: 0,
        });
        let mut events = Vec::new();

        process_end_of_turn(&mut state, &mut events);
        assert_eq!(state.combatants[0].health, 93);
        assert!(state.combatants[0].statuses.is_empty());
        assert!(consume_stun(&mut state.combatants[1], &mut events));
        assert!(state.combatants[1].statuses.is_empty());
        assert!(events.iter().any(|event| matches!(
            event,
            CombatEvent::StatusPreventedAction { pet: id, .. } if *id == other
        )));
    }
}
