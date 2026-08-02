use application::{AppError, Command};
use bevy::prelude::*;
use domain::events::DomainEvent;
use domain::identifiers::SaveSlotId;

#[derive(Debug, Message)]
pub struct AppCommand(pub Command);

#[derive(Clone, Debug, Message)]
pub struct DomainEventMessage(pub DomainEvent);

#[derive(Clone, Debug, Message)]
pub struct CommandFailed {
    pub(crate) message: String,
    pub(crate) user_rejection: bool,
}

#[derive(Clone, Copy, Debug, Message)]
pub struct SwitchSaveSlot(pub SaveSlotId);

impl From<AppError> for CommandFailed {
    fn from(error: AppError) -> Self {
        Self {
            message: error.to_string(),
            user_rejection: false,
        }
    }
}

impl CommandFailed {
    pub(crate) fn user(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            user_rejection: true,
        }
    }

    pub(crate) fn for_command(command: &Command, error: AppError) -> Self {
        let friendly = match (command, &error) {
            (Command::FeedPet { .. }, AppError::Domain(detail))
                if detail.ends_with("is not hungry") =>
            {
                Some("No feeding needed - your pet is already full.")
            }
            (Command::FeedPet { .. }, AppError::Domain(detail))
                if detail.contains("is not in inventory") =>
            {
                Some("You do not have an apple slice available.")
            }
            (Command::FeedPet { .. }, AppError::Domain(detail)) if detail.ends_with("is dead") => {
                Some("This pet can no longer be fed.")
            }
            (Command::Breed { .. }, AppError::Domain(detail))
                if detail.ends_with("is not an adult") =>
            {
                Some("Both parents must be healthy adults at least 7 days old.")
            }
            (Command::Breed { .. }, AppError::Domain(detail))
                if detail.ends_with("is not alive") =>
            {
                Some("Both parents must be alive to breed.")
            }
            (Command::Breed { .. }, AppError::Domain(detail)) if detail.ends_with("is ill") => {
                Some("An ill pet must recover before breeding.")
            }
            (Command::Breed { .. }, AppError::Domain(detail))
                if detail.ends_with("is still on breeding cooldown") =>
            {
                Some("A parent is still resting after breeding.")
            }
            (Command::Breed { .. }, AppError::Domain(detail))
                if detail == "a pet cannot breed with itself" =>
            {
                Some("Choose two different parents.")
            }
            (Command::Breed { .. }, AppError::Domain(detail))
                if detail == "direct ancestors cannot breed"
                    || detail.starts_with("pair is too closely related") =>
            {
                Some("These pets are too closely related to breed.")
            }
            (Command::CreatePet { .. }, AppError::Domain(detail))
                if detail.starts_with("all ") && detail.contains("adoption slots are full") =>
            {
                Some("Adoption slots are full. Unlock another with coins or release a pet.")
            }
            (Command::UnlockAdoptionSlot, AppError::Domain(detail))
                if detail.starts_with("you need ") =>
            {
                Some("Not enough coins yet. Win battles to fund this upgrade.")
            }
            (Command::UnlockAdoptionSlot, AppError::Domain(detail))
                if detail == "all adoption slots are already unlocked" =>
            {
                Some("All adoption slots are already unlocked.")
            }
            (Command::ReleasePet { .. }, AppError::Domain(detail))
                if detail == "a pet in an active battle cannot be released" =>
            {
                Some("Finish this pet's active battle before releasing it.")
            }
            (Command::ReleasePet { .. }, AppError::Domain(detail))
                if detail == "a pet with an unhatched egg cannot be released" =>
            {
                Some("Hatch this pet's pending egg before releasing it.")
            }
            _ => None,
        };
        friendly.map_or_else(
            || error.into(),
            |message| Self {
                message: message.into(),
                user_rejection: true,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use domain::identifiers::{ItemId, PetId};
    use domain::rng::WorldSeed;
    use domain::time::Tick;

    use super::*;

    #[test]
    fn expected_feed_rejections_hide_internal_identifiers() {
        let pet = PetId::derive(WorldSeed(1), Tick::ZERO, 1);
        let command = Command::FeedPet {
            pet,
            item: ItemId::new("apple_slice").unwrap(),
        };
        let failure = CommandFailed::for_command(
            &command,
            AppError::Domain(format!("pet `{pet}` is not hungry")),
        );

        assert!(failure.user_rejection);
        assert_eq!(
            failure.message,
            "No feeding needed - your pet is already full."
        );
        assert!(!failure.message.contains(&pet.to_string()));
    }

    #[test]
    fn expected_breeding_rejections_hide_internal_identifiers() {
        let first = PetId::derive(WorldSeed(1), Tick::ZERO, 1);
        let second = PetId::derive(WorldSeed(1), Tick::ZERO, 2);
        let command = Command::Breed { first, second };
        let failure = CommandFailed::for_command(
            &command,
            AppError::Domain(format!("pet {first} is not an adult")),
        );

        assert!(failure.user_rejection);
        assert_eq!(
            failure.message,
            "Both parents must be healthy adults at least 7 days old."
        );
        assert!(!failure.message.contains(&first.to_string()));
    }
}
