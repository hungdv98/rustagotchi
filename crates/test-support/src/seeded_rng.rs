//! Fixed seeds for tests.
//!
//! Tests never invent randomness. They pick a seed, and a failure is
//! reproducible by rerunning with the same one.

use domain::rng::{Rng, RngStream, WorldSeed, rng_for};
use domain::time::Tick;

/// The seed used by tests that do not care which world they are in.
pub const TEST_SEED: WorldSeed = WorldSeed(0x5253_4147_4F54_4348);

/// A generator at the origin of the given stream.
#[must_use]
pub fn test_rng(stream: RngStream) -> Rng {
    rng_for(TEST_SEED, stream, Tick::ZERO, 0)
}

/// A generator at an explicit position, for tests that care about isolation
/// between ticks or entities.
#[must_use]
pub fn test_rng_at(stream: RngStream, tick: u64, entity: u64) -> Rng {
    rng_for(TEST_SEED, stream, Tick(tick), entity)
}

/// Collects `count` raw draws. Useful for asserting that two positions differ,
/// or that one position is stable.
#[must_use]
pub fn draws(stream: RngStream, tick: u64, entity: u64, count: usize) -> Vec<u64> {
    let mut rng = test_rng_at(stream, tick, entity);
    (0..count).map(|_| rng.next_u64()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_position_is_reproducible() {
        assert_eq!(
            draws(RngStream::Mutation, 1, 1, 8),
            draws(RngStream::Mutation, 1, 1, 8)
        );
    }

    #[test]
    fn different_positions_diverge() {
        let base = draws(RngStream::Mutation, 1, 1, 8);
        assert_ne!(base, draws(RngStream::Mutation, 2, 1, 8));
        assert_ne!(base, draws(RngStream::Mutation, 1, 2, 8));
        assert_ne!(base, draws(RngStream::CombatHit, 1, 1, 8));
    }
}
