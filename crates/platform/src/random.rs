use application::SeedError;
use application::ports::RandomSource;
use domain::rng::WorldSeed;

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemRandomSource;

impl RandomSource for SystemRandomSource {
    fn world_seed(&self) -> Result<WorldSeed, SeedError> {
        let mut bytes = [0u8; 8];
        getrandom::fill(&mut bytes).map_err(|error| SeedError(error.to_string()))?;
        Ok(WorldSeed(u64::from_le_bytes(bytes)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_produces_seed_material() {
        let first = SystemRandomSource.world_seed().unwrap();
        let second = SystemRandomSource.world_seed().unwrap();
        assert_ne!(first, second);
    }
}
