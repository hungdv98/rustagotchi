use serde::{Deserialize, Serialize};

use crate::identifiers::{EggId, PetId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BreedingEvent {
    EggCreated { egg: EggId, parents: [PetId; 2] },
    EggHatched { egg: EggId, pet: PetId },
}
