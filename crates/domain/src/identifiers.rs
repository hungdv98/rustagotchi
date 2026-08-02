//! Identifier newtypes.
//!
//! Nothing in this codebase passes a bare `Uuid` or `String` as an identity.
//! Two `Uuid` parameters next to each other are a bug waiting to happen; two
//! distinct newtypes are a compile error.
//!
//! There are two families:
//!
//! * **Runtime ids** ([`PetId`], [`EggId`], [`BattleId`], [`SaveSlotId`]) name
//!   things the game creates. They are *derived*, not random — see below.
//! * **Content ids** ([`SpeciesId`], [`ItemId`], …) name things a designer
//!   authored. They wrap a validated [`ContentKey`].
//!
//! # Why runtime ids are derived and not v4
//!
//! ADR 0007 removes ambient randomness from the core, and `Uuid::new_v4` is
//! exactly that: it reads OS entropy, so the same world replayed from the same
//! seed would produce different ids and every equivalence test would fail. Ids
//! are instead derived from `(world seed, tick, ordinal)`, which makes a replay
//! reproduce the identical id graph. This is why the workspace `uuid`
//! dependency does not enable the `v4` feature.
//!
//! The result is a UUIDv8 — the version reserved for exactly this,
//! implementation-defined deterministic construction.

use core::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::rng::{WorldSeed, derive_seed_bytes};
use crate::time::Tick;
use crate::validation::{ValidationError, require_identifier};

/// A validated `snake_case` content identifier.
///
/// Cannot be constructed from an unvalidated string, including through
/// `serde` — deserialization runs the same grammar check, so a malformed id in
/// a content pack or a save is rejected at the boundary rather than surfacing
/// later as a failed lookup.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct ContentKey(Box<str>);

impl ContentKey {
    pub const MAX_LENGTH: usize = 64;

    /// # Errors
    ///
    /// If `raw` is not a valid `snake_case` identifier of at most
    /// [`Self::MAX_LENGTH`] bytes.
    pub fn new(field: &'static str, raw: &str) -> Result<Self, ValidationError> {
        require_identifier(field, raw, Self::MAX_LENGTH)?;
        Ok(ContentKey(raw.into()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ContentKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ContentKey {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        ContentKey::new("content id", &raw).map_err(serde::de::Error::custom)
    }
}

/// Declares a content identifier newtype over [`ContentKey`].
macro_rules! content_id {
    ($(#[$attributes:meta])* $name:ident, $field:literal) => {
        $(#[$attributes])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(ContentKey);

        impl $name {
            #[doc = concat!("Validates `raw` as a ", $field, ".")]
            ///
            /// # Errors
            ///
            /// If `raw` is not a valid `snake_case` identifier.
            pub fn new(raw: &str) -> Result<Self, ValidationError> {
                Ok($name(ContentKey::new($field, raw)?))
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }

            #[must_use]
            pub fn key(&self) -> &ContentKey {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0, f)
            }
        }
    };
}

content_id!(
    /// Names a species, e.g. `emberling`.
    SpeciesId,
    "species id"
);
content_id!(
    /// Names an item definition, e.g. `berry_snack`.
    ItemId,
    "item id"
);
content_id!(
    /// Names a combat move, e.g. `ember_bite`.
    MoveId,
    "move id"
);
content_id!(
    /// Names an illness, e.g. `sniffles`.
    IllnessId,
    "illness id"
);
content_id!(
    /// Names a quest.
    QuestId,
    "quest id"
);
content_id!(
    /// Names an achievement.
    AchievementId,
    "achievement id"
);
content_id!(
    /// Names a room.
    RoomId,
    "room id"
);
content_id!(
    /// Names a genetic locus, e.g. `coat_colour`.
    GeneId,
    "gene id"
);

/// Declares a derived runtime identifier newtype over [`Uuid`].
macro_rules! entity_id {
    ($(#[$attributes:meta])* $name:ident, $tag:literal) => {
        $(#[$attributes])*
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            /// A distinct constant per id type, so that two kinds of id minted
            /// at the same `(tick, ordinal)` never collide.
            const TAG: u64 = $tag;

            /// Derives an id from its position in the world's history.
            ///
            /// `ordinal` is a per-world monotonic counter. The same
            /// `(seed, tick, ordinal)` always yields the same id, which is what
            /// lets a replay reproduce the identical id graph.
            #[must_use]
            pub fn derive(seed: WorldSeed, tick: Tick, ordinal: u64) -> Self {
                $name(derive_uuid(Self::TAG, seed, tick, ordinal))
            }

            /// Reconstructs an id from stored bytes. For deserialization and
            /// tests; prefer [`Self::derive`] when minting a new one.
            #[must_use]
            pub const fn from_uuid(uuid: Uuid) -> Self {
                $name(uuid)
            }

            #[must_use]
            pub const fn as_uuid(self) -> Uuid {
                self.0
            }

            /// The `entity` argument for [`crate::rng::rng_for`].
            #[must_use]
            pub fn rng_key(self) -> u64 {
                let bytes = self.0.as_bytes();
                let high = u64::from_le_bytes(
                    bytes[..8].try_into().expect("uuid is 16 bytes"),
                );
                let low = u64::from_le_bytes(
                    bytes[8..].try_into().expect("uuid is 16 bytes"),
                );
                high ^ low
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl core::str::FromStr for $name {
            type Err = uuid::Error;

            fn from_str(raw: &str) -> Result<Self, Self::Err> {
                Uuid::parse_str(raw).map($name)
            }
        }
    };
}

entity_id!(
    /// Identifies a pet for the lifetime of a world.
    PetId,
    0x5065_7400_0000_0001
);

#[cfg(test)]
mod entity_id_tests {
    use super::*;

    #[test]
    fn runtime_ids_round_trip_through_their_display_form() {
        let original = PetId::derive(WorldSeed(12), Tick(34), 56);
        let parsed: PetId = original.to_string().parse().unwrap();
        assert_eq!(parsed, original);
    }

    #[test]
    fn runtime_ids_reject_malformed_text() {
        assert!("not-a-uuid".parse::<PetId>().is_err());
    }
}
entity_id!(
    /// Identifies an egg, from creation until it hatches.
    EggId,
    0x4567_6700_0000_0002
);
entity_id!(
    /// Identifies one battle.
    BattleId,
    0x4261_7400_0000_0003
);
entity_id!(
    /// Identifies a save slot.
    SaveSlotId,
    0x536C_6F74_0000_0004
);

/// Builds a version 8 UUID from the derivation inputs.
///
/// Version 8 is the UUID variant reserved for implementation-defined
/// construction, which is exactly what this is. Setting the version and variant
/// bits keeps the value a well-formed UUID for any tool that inspects a save.
fn derive_uuid(tag: u64, seed: WorldSeed, tick: Tick, ordinal: u64) -> Uuid {
    let bytes = derive_seed_bytes(seed.0, tag, tick.0, ordinal);
    let mut uuid_bytes = [0u8; 16];
    uuid_bytes.copy_from_slice(&bytes[..16]);
    uuid_bytes[6] = (uuid_bytes[6] & 0x0F) | 0x80; // version 8
    uuid_bytes[8] = (uuid_bytes[8] & 0x3F) | 0x80; // RFC 4122 variant
    Uuid::from_bytes(uuid_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEED: WorldSeed = WorldSeed(0x1234_5678);

    #[test]
    fn derivation_is_reproducible() {
        let first = PetId::derive(SEED, Tick(10), 3);
        let second = PetId::derive(SEED, Tick(10), 3);
        assert_eq!(first, second, "a replay must reproduce the identical id");
    }

    #[test]
    fn every_input_changes_the_id() {
        let base = PetId::derive(SEED, Tick(10), 3);
        assert_ne!(base, PetId::derive(WorldSeed(0x1234_5679), Tick(10), 3));
        assert_ne!(base, PetId::derive(SEED, Tick(11), 3));
        assert_ne!(base, PetId::derive(SEED, Tick(10), 4));
    }

    #[test]
    fn different_id_types_do_not_collide_at_the_same_position() {
        // Without the per-type tag, a pet and an egg minted on the same tick
        // with the same ordinal would share a UUID.
        let pet = PetId::derive(SEED, Tick(10), 3).as_uuid();
        let egg = EggId::derive(SEED, Tick(10), 3).as_uuid();
        let battle = BattleId::derive(SEED, Tick(10), 3).as_uuid();
        assert_ne!(pet, egg);
        assert_ne!(pet, battle);
        assert_ne!(egg, battle);
    }

    #[test]
    fn derived_ids_are_well_formed_v8_uuids() {
        let uuid = PetId::derive(SEED, Tick(1), 1).as_uuid();
        assert_eq!(uuid.get_version_num(), 8);
        assert_eq!(uuid.get_variant(), uuid::Variant::RFC4122);
    }

    #[test]
    fn ordinals_do_not_collide_across_a_long_run() {
        let ids: std::collections::BTreeSet<_> = (0..5_000)
            .map(|ordinal| PetId::derive(SEED, Tick(7), ordinal))
            .collect();
        assert_eq!(ids.len(), 5_000, "derived ids must be distinct per ordinal");
    }

    #[test]
    fn rng_key_is_stable_and_distinguishes_pets() {
        let first = PetId::derive(SEED, Tick(1), 1);
        let second = PetId::derive(SEED, Tick(1), 2);
        assert_eq!(first.rng_key(), first.rng_key());
        assert_ne!(first.rng_key(), second.rng_key());
    }

    #[test]
    fn ids_survive_a_serde_round_trip() {
        let pet = PetId::derive(SEED, Tick(1), 1);
        let encoded = serde_json::to_string(&pet).expect("serialize");
        let decoded: PetId = serde_json::from_str(&encoded).expect("deserialize");
        assert_eq!(pet, decoded);
    }

    #[test]
    fn content_ids_accept_snake_case() {
        assert_eq!(SpeciesId::new("emberling").unwrap().as_str(), "emberling");
        assert_eq!(
            MoveId::new("ember_bite_2").unwrap().as_str(),
            "ember_bite_2"
        );
    }

    #[test]
    fn content_ids_reject_malformed_input() {
        assert!(SpeciesId::new("Emberling").is_err());
        assert!(SpeciesId::new("").is_err());
        assert!(SpeciesId::new("ember ling").is_err());
        assert!(ItemId::new(&"a".repeat(65)).is_err());
    }

    #[test]
    fn content_ids_validate_on_deserialize_too() {
        // The grammar has to hold at the boundary, not just at the constructor,
        // or a hand-edited content pack smuggles a bad id straight through.
        let ok: Result<SpeciesId, _> = serde_json::from_str("\"emberling\"");
        assert!(ok.is_ok());

        let bad: Result<SpeciesId, _> = serde_json::from_str("\"Ember Ling\"");
        assert!(bad.is_err());
    }

    #[test]
    fn content_id_types_are_not_interchangeable() {
        // A compile-time property, asserted here only as documentation: the
        // following would not compile.
        //     let species: SpeciesId = ItemId::new("berry").unwrap();
        let species = SpeciesId::new("emberling").unwrap();
        let item = ItemId::new("emberling").unwrap();
        assert_eq!(species.as_str(), item.as_str());
    }
}
