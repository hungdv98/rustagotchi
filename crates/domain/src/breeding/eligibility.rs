use crate::pet::Pet;
use crate::time::{GameDuration, Tick};
use crate::units::Milli;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BreedingRules {
    pub minimum_age: GameDuration,
    pub cooldown: GameDuration,
    pub maximum_relatedness: Milli,
}

impl Default for BreedingRules {
    fn default() -> Self {
        Self {
            minimum_age: GameDuration::from_days(7),
            cooldown: GameDuration::from_days(1),
            maximum_relatedness: Milli(125),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BreedingEligibilityError {
    #[error("a pet cannot breed with itself")]
    SamePet,
    #[error("pet {0} is not alive")]
    Dead(crate::identifiers::PetId),
    #[error("pet {0} is not an adult")]
    TooYoung(crate::identifiers::PetId),
    #[error("pet {0} is ill")]
    Ill(crate::identifiers::PetId),
    #[error("pet {0} is still on breeding cooldown")]
    Cooldown(crate::identifiers::PetId),
    #[error("direct ancestors cannot breed")]
    DirectLine,
    #[error("pair is too closely related ({actual} > {maximum})")]
    TooCloselyRelated { actual: Milli, maximum: Milli },
}

pub fn validate_pair(
    first: &Pet,
    second: &Pet,
    now: Tick,
    last_bred: [Option<Tick>; 2],
    rules: BreedingRules,
) -> Result<(), BreedingEligibilityError> {
    if first.id == second.id {
        return Err(BreedingEligibilityError::SamePet);
    }
    for (pet, bred_at) in [(first, last_bred[0]), (second, last_bred[1])] {
        if !pet.is_alive() {
            return Err(BreedingEligibilityError::Dead(pet.id));
        }
        if !pet.stage.can_breed() || pet.progression.age(now) < rules.minimum_age {
            return Err(BreedingEligibilityError::TooYoung(pet.id));
        }
        if pet.health.is_ill() {
            return Err(BreedingEligibilityError::Ill(pet.id));
        }
        if bred_at.is_some_and(|tick| now.since(tick) < rules.cooldown) {
            return Err(BreedingEligibilityError::Cooldown(pet.id));
        }
    }
    if first
        .ancestry
        .is_direct_line(first.id, &second.ancestry, second.id)
    {
        return Err(BreedingEligibilityError::DirectLine);
    }
    let relatedness = first.ancestry.shared_ancestry(&second.ancestry);
    if relatedness > rules.maximum_relatedness {
        return Err(BreedingEligibilityError::TooCloselyRelated {
            actual: relatedness,
            maximum: rules.maximum_relatedness,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genetics::{Ancestry, GeneticProfile, Parentage};
    use crate::identifiers::{PetId, SpeciesId};
    use crate::pet::{LifeStage, Personality};
    use crate::rng::WorldSeed;

    fn adult(ordinal: u64) -> Pet {
        let seed = WorldSeed(1);
        let mut pet = Pet::hatch(
            PetId::derive(seed, Tick::ZERO, ordinal),
            SpeciesId::new("emberling").unwrap(),
            GeneticProfile::new(),
            Personality::neutral(),
            Parentage::Founder,
            Ancestry::founder(),
            Tick::ZERO,
        );
        pet.stage = LifeStage::Adult;
        pet
    }

    #[test]
    fn healthy_unrelated_adults_are_eligible() {
        assert!(
            validate_pair(
                &adult(1),
                &adult(2),
                Tick(20_000),
                [None, None],
                BreedingRules::default()
            )
            .is_ok()
        );
    }

    #[test]
    fn cooldown_and_self_pairing_are_refused() {
        let first = adult(1);
        assert_eq!(
            validate_pair(
                &first,
                &first,
                Tick(20_000),
                [None, None],
                BreedingRules::default()
            ),
            Err(BreedingEligibilityError::SamePet)
        );
        assert!(matches!(
            validate_pair(
                &first,
                &adult(2),
                Tick(20_000),
                [Some(Tick(19_999)), None],
                BreedingRules::default()
            ),
            Err(BreedingEligibilityError::Cooldown(_))
        ));
    }
}
