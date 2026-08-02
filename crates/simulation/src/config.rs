//! Data-driven simulation balance configuration.

use domain::pet::NeedKind;
use domain::time::{GameDuration, TICKS_PER_DAY};
use domain::units::Milli;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NeedDecayConfig {
    pub hunger: i32,
    pub energy: i32,
    pub hygiene: i32,
    pub bladder: i32,
    pub social: i32,
    pub fun: i32,
}

impl NeedDecayConfig {
    #[must_use]
    pub const fn get(&self, kind: NeedKind) -> Milli {
        Milli(match kind {
            NeedKind::Hunger => self.hunger,
            NeedKind::Energy => self.energy,
            NeedKind::Hygiene => self.hygiene,
            NeedKind::Bladder => self.bladder,
            NeedKind::Social => self.social,
            NeedKind::Fun => self.fun,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifeStageConfig {
    pub child_days: u64,
    pub teen_days: u64,
    pub adult_days: u64,
    pub elder_days: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SimConfig {
    pub need_decay_per_tick: NeedDecayConfig,
    pub neglect_damage_per_tick: i32,
    pub passive_experience_per_tick: u32,
    pub illness_roll_numerator_per_day: u32,
    pub illness_roll_denominator: u32,
    pub illness_damage_per_tick: i32,
    pub habit_decay_per_day: i32,
    pub memory_decay_per_day: i32,
    pub stages: LifeStageConfig,
}

impl SimConfig {
    pub fn from_ron(source: &str) -> Result<Self, ConfigError> {
        let config: Self =
            ron::from_str(source).map_err(|error| ConfigError::Parse(error.to_string()))?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        for kind in NeedKind::ALL {
            let rate = self.need_decay_per_tick.get(kind).0;
            if !(0..=100_000).contains(&rate) {
                return Err(ConfigError::Invalid(format!(
                    "{} decay must be 0..=100000",
                    kind.name()
                )));
            }
        }
        if self.neglect_damage_per_tick <= 0 || self.illness_damage_per_tick < 0 {
            return Err(ConfigError::Invalid(
                "damage rates must be non-negative and neglect damage must be positive".into(),
            ));
        }
        if self.illness_roll_denominator == 0
            || self.illness_roll_numerator_per_day > self.illness_roll_denominator
        {
            return Err(ConfigError::Invalid(
                "illness probability must be a valid fraction".into(),
            ));
        }
        let stages = [
            &self.stages.child_days,
            &self.stages.teen_days,
            &self.stages.adult_days,
            &self.stages.elder_days,
        ];
        if !stages.windows(2).all(|pair| pair[0] < pair[1]) {
            return Err(ConfigError::Invalid(
                "life-stage day thresholds must be strictly increasing".into(),
            ));
        }
        Ok(())
    }

    #[must_use]
    pub const fn stage_age(&self, days: u64) -> GameDuration {
        GameDuration::from_ticks(days.saturating_mul(TICKS_PER_DAY))
    }
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            need_decay_per_tick: NeedDecayConfig {
                hunger: 25,
                energy: 18,
                hygiene: 12,
                bladder: 20,
                social: 8,
                fun: 10,
            },
            neglect_damage_per_tick: 100,
            passive_experience_per_tick: 14,
            illness_roll_numerator_per_day: 1,
            illness_roll_denominator: 20,
            illness_damage_per_tick: 5,
            habit_decay_per_day: 50,
            memory_decay_per_day: 50,
            stages: LifeStageConfig {
                child_days: 1,
                teen_days: 3,
                adult_days: 7,
                elder_days: 365,
            },
        }
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("invalid simulation RON: {0}")]
    Parse(String),
    #[error("invalid simulation configuration: {0}")]
    Invalid(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_are_valid() {
        SimConfig::default().validate().unwrap();
    }
    #[test]
    fn refuses_invalid_probabilities() {
        let value = SimConfig {
            illness_roll_denominator: 0,
            ..SimConfig::default()
        };
        assert!(value.validate().is_err());
    }
}
