//! Deterministic randomness. See ADR 0007.
//!
//! Nothing in this game holds a random number generator. A generator is
//! *derived*, on demand, as a pure function of where you are in state space:
//!
//! ```
//! use domain::rng::{RngStream, WorldSeed, rng_for};
//! use domain::time::Tick;
//!
//! let mut rng = rng_for(WorldSeed(42), RngStream::Mutation, Tick(1_000), 7);
//! let mutated = rng.chance(1, 100);
//! ```
//!
//! Three properties follow, and all three are load-bearing:
//!
//! * **Nothing to save.** Generator state never enters the save file, so
//!   save/load equivalence is structural rather than something to test for.
//! * **Systems are isolated.** Adding an accuracy roll to combat cannot shift
//!   which pets mutate, because the two draw from different streams. Balance
//!   changes stay local.
//! * **Ticks are reproducible standalone.** A nightly failure at tick 400,000
//!   reduces to one `rng_for` call, not a replay of the preceding 399,999 ticks.
//!
//! The isolation guarantee is *between* `(stream, tick, entity)` cells. Within
//! one cell the draws are an ordinary sequence, so consuming a different number
//! of them still changes that cell's outcome.

use rand_chacha::ChaCha8Rng;
// rand_core 0.10 renamed `RngCore` to `Rng`. Imported anonymously so it does not
// collide with our own `Rng`, which is the only generator handle the rest of the
// codebase is allowed to see.
use rand_chacha::rand_core::{Rng as _, SeedableRng};
use serde::{Deserialize, Serialize};

use crate::time::Tick;

/// The root seed for a world. Generated once, from OS entropy, in
/// `crates/platform`; saved; and never regenerated.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct WorldSeed(pub u64);

/// An independent randomness stream.
///
/// Discriminants are part of the save-compatible contract: a world seeded today
/// must produce the same mutations after any number of releases. **Never
/// renumber an existing variant.** New kinds of roll take the next free number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u16)]
#[non_exhaustive]
pub enum RngStream {
    NeedsDecay = 1,
    IllnessOnset = 2,
    Mutation = 3,
    GeneInheritance = 4,
    CombatHit = 5,
    CombatCrit = 6,
    CombatAi = 7,
    LootRoll = 8,
    BehaviorChoice = 9,
    WeatherRoll = 10,
    QuestRoll = 11,
    /// Deriving stable identifiers. See `crate::identifiers`.
    IdentityMint = 12,
    CombatStatus = 13,
}

impl RngStream {
    #[must_use]
    pub const fn as_u16(self) -> u16 {
        self as u16
    }
}

/// A derived generator.
///
/// Deliberately not constructible from entropy and not `Clone` from thin air —
/// the only way to obtain one is [`rng_for`], which forces the caller to state
/// which stream and which position they are drawing from.
#[derive(Debug, Clone)]
pub struct Rng(ChaCha8Rng);

impl Rng {
    #[must_use]
    pub fn next_u64(&mut self) -> u64 {
        self.0.next_u64()
    }

    #[must_use]
    pub fn next_u32(&mut self) -> u32 {
        self.0.next_u32()
    }

    /// A uniform value in `0..bound`.
    ///
    /// Rejection-sampled, so the distribution is exactly uniform. The naive
    /// `next_u64() % bound` is biased toward small values, which over 100,000
    /// nightly battles is visible in the win-rate assertions.
    ///
    /// # Panics
    ///
    /// If `bound` is zero.
    #[must_use]
    pub fn below(&mut self, bound: u64) -> u64 {
        assert!(bound > 0, "Rng::below requires a non-zero bound");
        // The largest multiple of `bound` that fits; values at or above it would
        // skew the modulo, so they are redrawn.
        let limit = (u64::MAX / bound) * bound;
        loop {
            let value = self.next_u64();
            if value < limit {
                return value % bound;
            }
        }
    }

    /// A uniform value in `low..=high`.
    ///
    /// # Panics
    ///
    /// If `low > high`.
    #[must_use]
    pub fn range_inclusive(&mut self, low: i32, high: i32) -> i32 {
        assert!(
            low <= high,
            "Rng::range_inclusive called with an inverted range"
        );
        let span = (high as i64 - low as i64) as u64 + 1;
        (low as i64 + self.below(span) as i64) as i32
    }

    /// True with probability `numerator / denominator`.
    ///
    /// # Panics
    ///
    /// If `denominator` is zero.
    #[must_use]
    pub fn chance(&mut self, numerator: u32, denominator: u32) -> bool {
        assert!(
            denominator > 0,
            "Rng::chance requires a non-zero denominator"
        );
        self.below(u64::from(denominator)) < u64::from(numerator)
    }

    /// A uniformly chosen element, or `None` if the slice is empty.
    #[must_use]
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        if items.is_empty() {
            return None;
        }
        let index = self.below(items.len() as u64) as usize;
        items.get(index)
    }

    /// Fisher-Yates, so that shuffled order is reproducible from the seed.
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.below(i as u64 + 1) as usize;
            items.swap(i, j);
        }
    }
}

/// Derives the generator for one point in state space.
///
/// `entity` distinguishes actors within a tick — a [`crate::identifiers::PetId`]
/// contributes its `rng_key()`. Use `0` for rolls that belong to the world
/// rather than to any particular actor.
#[must_use]
pub fn rng_for(seed: WorldSeed, stream: RngStream, tick: Tick, entity: u64) -> Rng {
    Rng(ChaCha8Rng::from_seed(derive_seed_bytes(
        seed.0,
        u64::from(stream.as_u16()),
        tick.0,
        entity,
    )))
}

/// Expands four inputs into a 32-byte ChaCha key.
///
/// SplitMix64 finalisation: cheap, well-distributed, and — because it is written
/// out here rather than pulled from a crate — guaranteed not to change under us.
/// The output of this function is a permanent compatibility surface; the
/// stability test below pins it.
pub(crate) fn derive_seed_bytes(a: u64, b: u64, c: u64, d: u64) -> [u8; 32] {
    const GOLDEN: u64 = 0x9E37_79B9_7F4A_7C15;

    let mut accumulator = 0xA076_1D64_78BD_642F;
    for input in [a, b, c, d] {
        accumulator = mix(accumulator ^ input);
    }

    let mut bytes = [0u8; 32];
    for (index, chunk) in bytes.chunks_exact_mut(8).enumerate() {
        accumulator = mix(accumulator ^ (index as u64).wrapping_mul(GOLDEN));
        chunk.copy_from_slice(&accumulator.to_le_bytes());
    }
    bytes
}

const fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draws(stream: RngStream, tick: u64, entity: u64) -> Vec<u64> {
        let mut rng = rng_for(WorldSeed(0xDEAD_BEEF), stream, Tick(tick), entity);
        (0..4).map(|_| rng.next_u64()).collect()
    }

    /// Locks the generator's output forever.
    ///
    /// If this test fails, every existing world would produce different
    /// mutations, hatches and battle outcomes than it did before. That is a save
    /// compatibility break, not a test to update. The only legitimate reason to
    /// change these numbers is a deliberate, documented world-format bump.
    #[test]
    fn derived_output_is_stable_across_releases() {
        assert_eq!(
            draws(RngStream::Mutation, 1_000, 7),
            [
                5_783_174_119_569_774_862,
                10_911_444_281_265_470_309,
                7_132_285_750_161_560_247,
                1_910_806_613_179_424_767,
            ]
        );
    }

    #[test]
    fn streams_are_independent_at_the_same_position() {
        let mutation = draws(RngStream::Mutation, 100, 1);
        let combat = draws(RngStream::CombatHit, 100, 1);
        assert_ne!(
            mutation, combat,
            "streams must not alias at the same position"
        );
    }

    #[test]
    fn ticks_and_entities_are_independent() {
        let base = draws(RngStream::NeedsDecay, 100, 1);
        assert_ne!(base, draws(RngStream::NeedsDecay, 101, 1));
        assert_ne!(base, draws(RngStream::NeedsDecay, 100, 2));
    }

    #[test]
    fn the_same_position_always_yields_the_same_draws() {
        assert_eq!(
            draws(RngStream::LootRoll, 55, 9),
            draws(RngStream::LootRoll, 55, 9)
        );
    }

    #[test]
    fn stream_discriminants_are_frozen() {
        // Renumbering any of these silently rewrites the history of every save.
        assert_eq!(RngStream::NeedsDecay.as_u16(), 1);
        assert_eq!(RngStream::IllnessOnset.as_u16(), 2);
        assert_eq!(RngStream::Mutation.as_u16(), 3);
        assert_eq!(RngStream::GeneInheritance.as_u16(), 4);
        assert_eq!(RngStream::CombatHit.as_u16(), 5);
        assert_eq!(RngStream::CombatCrit.as_u16(), 6);
        assert_eq!(RngStream::CombatAi.as_u16(), 7);
        assert_eq!(RngStream::LootRoll.as_u16(), 8);
        assert_eq!(RngStream::BehaviorChoice.as_u16(), 9);
        assert_eq!(RngStream::WeatherRoll.as_u16(), 10);
        assert_eq!(RngStream::QuestRoll.as_u16(), 11);
        assert_eq!(RngStream::IdentityMint.as_u16(), 12);
    }

    #[test]
    fn below_stays_in_range_and_covers_it() {
        let mut rng = rng_for(WorldSeed(1), RngStream::BehaviorChoice, Tick(0), 0);
        let mut seen = [false; 6];
        for _ in 0..2_000 {
            let value = rng.below(6);
            assert!(value < 6);
            seen[value as usize] = true;
        }
        assert!(
            seen.iter().all(|hit| *hit),
            "every outcome should appear over 2000 draws"
        );
    }

    #[test]
    fn below_one_is_always_zero() {
        let mut rng = rng_for(WorldSeed(1), RngStream::BehaviorChoice, Tick(0), 0);
        for _ in 0..100 {
            assert_eq!(rng.below(1), 0);
        }
    }

    #[test]
    fn below_is_close_to_uniform() {
        // A modulo-biased implementation skews low; 20k draws over 3 buckets
        // makes that obvious well outside the noise band.
        let mut rng = rng_for(WorldSeed(99), RngStream::CombatCrit, Tick(0), 0);
        let mut counts = [0u32; 3];
        for _ in 0..20_000 {
            counts[rng.below(3) as usize] += 1;
        }
        for count in counts {
            assert!(
                (6_000..=7_400).contains(&count),
                "bucket {count} is skewed: {counts:?}"
            );
        }
    }

    #[test]
    fn range_inclusive_covers_both_endpoints() {
        let mut rng = rng_for(WorldSeed(7), RngStream::CombatAi, Tick(3), 1);
        let (mut low, mut high) = (false, false);
        for _ in 0..500 {
            let value = rng.range_inclusive(-2, 2);
            assert!((-2..=2).contains(&value));
            low |= value == -2;
            high |= value == 2;
        }
        assert!(low && high);
    }

    #[test]
    fn range_inclusive_handles_a_single_value() {
        let mut rng = rng_for(WorldSeed(7), RngStream::CombatAi, Tick(3), 1);
        assert_eq!(rng.range_inclusive(5, 5), 5);
    }

    #[test]
    fn range_inclusive_spans_the_full_i32_domain() {
        // low..=high can exceed u32::MAX values; the i64 intermediate handles it.
        let mut rng = rng_for(WorldSeed(7), RngStream::CombatAi, Tick(3), 1);
        let value = rng.range_inclusive(i32::MIN, i32::MAX);
        let _ = value; // any value is in range by construction; this must not panic
    }

    #[test]
    fn chance_respects_its_bounds() {
        let mut rng = rng_for(WorldSeed(3), RngStream::IllnessOnset, Tick(0), 0);
        for _ in 0..100 {
            assert!(!rng.chance(0, 100), "a 0% chance must never fire");
            assert!(rng.chance(100, 100), "a 100% chance must always fire");
        }
    }

    #[test]
    fn pick_returns_none_for_an_empty_slice() {
        let mut rng = rng_for(WorldSeed(3), RngStream::LootRoll, Tick(0), 0);
        let empty: [u8; 0] = [];
        assert!(rng.pick(&empty).is_none());
    }

    #[test]
    fn shuffle_is_reproducible_and_preserves_contents() {
        let shuffled = |seed: u64| {
            let mut rng = rng_for(WorldSeed(seed), RngStream::BehaviorChoice, Tick(0), 0);
            let mut items: Vec<u8> = (0..16).collect();
            rng.shuffle(&mut items);
            items
        };

        let first = shuffled(1);
        assert_eq!(
            first,
            shuffled(1),
            "the same seed must shuffle the same way"
        );
        assert_ne!(first, shuffled(2));

        let mut sorted = first.clone();
        sorted.sort_unstable();
        assert_eq!(
            sorted,
            (0..16).collect::<Vec<u8>>(),
            "shuffle must not lose elements"
        );
    }

    #[test]
    #[should_panic(expected = "non-zero bound")]
    fn below_rejects_a_zero_bound() {
        let mut rng = rng_for(WorldSeed(1), RngStream::LootRoll, Tick(0), 0);
        let _ = rng.below(0);
    }
}
