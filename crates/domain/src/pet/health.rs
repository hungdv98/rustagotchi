//! Health and illness.

use core::fmt;

use serde::{Deserialize, Serialize};

use crate::identifiers::IllnessId;
use crate::time::Tick;
use crate::units::Milli;

/// How bad an illness is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Severity {
    Mild,
    Moderate,
    Severe,
}

impl Severity {
    pub const ALL: [Severity; 3] = [Severity::Mild, Severity::Moderate, Severity::Severe];

    /// The next stage, or `None` if already at the worst.
    #[must_use]
    pub const fn worsened(self) -> Option<Self> {
        match self {
            Severity::Mild => Some(Severity::Moderate),
            Severity::Moderate => Some(Severity::Severe),
            Severity::Severe => None,
        }
    }

    #[must_use]
    pub const fn eased(self) -> Option<Self> {
        match self {
            Severity::Severe => Some(Severity::Moderate),
            Severity::Moderate => Some(Severity::Mild),
            Severity::Mild => None,
        }
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Severity::Mild => "mild",
            Severity::Moderate => "moderate",
            Severity::Severe => "severe",
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// One illness a pet currently has.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ailment {
    pub illness: IllnessId,
    pub severity: Severity,
    pub contracted_at: Tick,
}

/// Health points and current illnesses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthState {
    current: Milli,
    ailments: Vec<Ailment>,
}

impl HealthState {
    pub const MAX: Milli = Milli(100_000);

    /// A pet cannot have more illnesses than this at once.
    ///
    /// Without a cap, an illness system with a per-tick onset roll can pile up
    /// unbounded entries in a long offline catch-up — a save that grows without
    /// limit and a UI that cannot render it.
    pub const MAX_AILMENTS: usize = 8;

    #[must_use]
    pub fn healthy() -> Self {
        HealthState {
            current: Self::MAX,
            ailments: Vec::new(),
        }
    }

    #[must_use]
    pub const fn current(&self) -> Milli {
        self.current
    }

    #[must_use]
    pub const fn is_alive(&self) -> bool {
        self.current.0 > 0
    }

    #[must_use]
    pub fn fraction(&self) -> Milli {
        self.current.ratio_of(Self::MAX)
    }

    /// Reduces health, flooring at zero.
    pub const fn damage(&mut self, amount: Milli) {
        self.current = self
            .current
            .saturating_sub(amount)
            .clamp_to(Milli::ZERO, Self::MAX);
    }

    /// Restores health, capping at [`Self::MAX`].
    ///
    /// A dead pet stays dead: healing checks `is_alive` first, because
    /// resurrection by a stray heal-over-time tick is not a feature.
    pub const fn heal(&mut self, amount: Milli) {
        if !self.is_alive() {
            return;
        }
        self.current = self
            .current
            .saturating_add(amount)
            .clamp_to(Milli::ZERO, Self::MAX);
    }

    #[must_use]
    pub fn ailments(&self) -> &[Ailment] {
        &self.ailments
    }

    #[must_use]
    pub fn is_ill(&self) -> bool {
        !self.ailments.is_empty()
    }

    #[must_use]
    pub fn has(&self, illness: &IllnessId) -> bool {
        self.ailments
            .iter()
            .any(|ailment| &ailment.illness == illness)
    }

    /// Contracts an illness.
    ///
    /// Returns `false` if the pet already has it or is at the cap — catching the
    /// same cold twice is not worse than catching it once, and the caller can
    /// use the answer to decide whether to emit an event.
    pub fn contract(&mut self, illness: IllnessId, severity: Severity, now: Tick) -> bool {
        if self.has(&illness) || self.ailments.len() >= Self::MAX_AILMENTS {
            return false;
        }
        self.ailments.push(Ailment {
            illness,
            severity,
            contracted_at: now,
        });
        // Kept sorted by illness id so iteration, saves and UI lists are stable
        // regardless of the order illnesses were caught in.
        self.ailments
            .sort_by(|left, right| left.illness.cmp(&right.illness));
        true
    }

    /// Removes an illness. Returns whether it was there.
    pub fn cure(&mut self, illness: &IllnessId) -> bool {
        let before = self.ailments.len();
        self.ailments.retain(|ailment| &ailment.illness != illness);
        self.ailments.len() != before
    }

    /// Advances one illness to the next severity. Returns the new severity, or
    /// `None` if it was absent or already at its worst.
    pub fn worsen(&mut self, illness: &IllnessId) -> Option<Severity> {
        let ailment = self
            .ailments
            .iter_mut()
            .find(|ailment| &ailment.illness == illness)?;
        let worse = ailment.severity.worsened()?;
        ailment.severity = worse;
        Some(worse)
    }

    /// The worst severity currently present.
    #[must_use]
    pub fn worst_severity(&self) -> Option<Severity> {
        self.ailments.iter().map(|ailment| ailment.severity).max()
    }
}

impl Default for HealthState {
    fn default() -> Self {
        Self::healthy()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn illness(name: &str) -> IllnessId {
        IllnessId::new(name).expect("valid illness id")
    }

    #[test]
    fn a_new_pet_is_healthy() {
        let health = HealthState::healthy();
        assert!(health.is_alive());
        assert!(!health.is_ill());
        assert_eq!(health.current(), HealthState::MAX);
        assert_eq!(health.fraction(), Milli::ONE);
    }

    #[test]
    fn damage_floors_at_zero_and_kills() {
        let mut health = HealthState::healthy();
        health.damage(Milli(i32::MAX));
        assert_eq!(health.current(), Milli::ZERO);
        assert!(!health.is_alive());
    }

    #[test]
    fn healing_caps_at_maximum() {
        let mut health = HealthState::healthy();
        health.damage(Milli(1_000));
        health.heal(Milli(i32::MAX));
        assert_eq!(health.current(), HealthState::MAX);
    }

    #[test]
    fn a_dead_pet_cannot_be_healed_back() {
        let mut health = HealthState::healthy();
        health.damage(HealthState::MAX);
        health.heal(Milli(50_000));
        assert!(!health.is_alive(), "healing must not resurrect");
    }

    #[test]
    fn contracting_and_curing_round_trip() {
        let mut health = HealthState::healthy();
        assert!(health.contract(illness("sniffles"), Severity::Mild, Tick(1)));
        assert!(health.is_ill());
        assert!(health.has(&illness("sniffles")));

        assert!(health.cure(&illness("sniffles")));
        assert!(!health.is_ill());
        assert!(
            !health.cure(&illness("sniffles")),
            "curing twice reports nothing to do"
        );
    }

    #[test]
    fn the_same_illness_cannot_be_caught_twice() {
        let mut health = HealthState::healthy();
        assert!(health.contract(illness("sniffles"), Severity::Mild, Tick(1)));
        assert!(!health.contract(illness("sniffles"), Severity::Severe, Tick(2)));
        assert_eq!(health.ailments().len(), 1);
    }

    #[test]
    fn ailments_are_capped() {
        let mut health = HealthState::healthy();
        for index in 0..HealthState::MAX_AILMENTS + 5 {
            health.contract(
                illness(&format!("illness_{index}")),
                Severity::Mild,
                Tick(1),
            );
        }
        assert_eq!(health.ailments().len(), HealthState::MAX_AILMENTS);
    }

    #[test]
    fn ailments_are_ordered_regardless_of_when_they_were_caught() {
        let mut first = HealthState::healthy();
        first.contract(illness("zeta_fever"), Severity::Mild, Tick(1));
        first.contract(illness("alpha_cough"), Severity::Mild, Tick(2));

        let mut second = HealthState::healthy();
        second.contract(illness("alpha_cough"), Severity::Mild, Tick(1));
        second.contract(illness("zeta_fever"), Severity::Mild, Tick(2));

        let names = |health: &HealthState| {
            health
                .ailments()
                .iter()
                .map(|a| a.illness.as_str().to_owned())
                .collect::<Vec<_>>()
        };
        assert_eq!(names(&first), ["alpha_cough", "zeta_fever"]);
        assert_eq!(names(&first), names(&second));
    }

    #[test]
    fn severity_escalates_and_stops_at_severe() {
        let mut health = HealthState::healthy();
        health.contract(illness("flu"), Severity::Mild, Tick(1));
        assert_eq!(health.worsen(&illness("flu")), Some(Severity::Moderate));
        assert_eq!(health.worsen(&illness("flu")), Some(Severity::Severe));
        assert_eq!(
            health.worsen(&illness("flu")),
            None,
            "severe is the ceiling"
        );
    }

    #[test]
    fn worsening_an_absent_illness_does_nothing() {
        let mut health = HealthState::healthy();
        assert_eq!(health.worsen(&illness("flu")), None);
    }

    #[test]
    fn severity_transitions_are_symmetric() {
        for severity in Severity::ALL {
            if let Some(worse) = severity.worsened() {
                assert_eq!(worse.eased(), Some(severity));
            }
        }
    }

    #[test]
    fn the_worst_severity_is_reported() {
        let mut health = HealthState::healthy();
        assert_eq!(health.worst_severity(), None);
        health.contract(illness("cough"), Severity::Mild, Tick(1));
        health.contract(illness("fever"), Severity::Severe, Tick(1));
        assert_eq!(health.worst_severity(), Some(Severity::Severe));
    }
}
