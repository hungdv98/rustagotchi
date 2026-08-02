//! Physical needs.
//!
//! Every need is a **satisfaction** level, not a deficit: [`NeedValue::FULL`]
//! means the pet is content and [`NeedValue::EMPTY`] means it is in trouble.
//! That includes the ones where the English is misleading — a full `bladder`
//! need means a comfortable pet, not a desperate one. One consistent direction
//! across all six needs means decay, restoration, mood weighting and the
//! critical check are each written once instead of six times with sign flips.

use core::fmt;

use serde::{Deserialize, Serialize};

use crate::units::Milli;

/// Which need. Used to address [`PhysicalNeeds`] generically and to tag events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum NeedKind {
    Hunger,
    Energy,
    Hygiene,
    Bladder,
    Social,
    Fun,
}

impl NeedKind {
    /// Every need, in a fixed order.
    ///
    /// The order is part of the determinism contract: anything that scans needs
    /// — decay, the lowest-need query, utility AI scoring — must visit them in
    /// this sequence so that ties break identically on every machine and in
    /// every replay.
    pub const ALL: [NeedKind; 6] = [
        NeedKind::Hunger,
        NeedKind::Energy,
        NeedKind::Hygiene,
        NeedKind::Bladder,
        NeedKind::Social,
        NeedKind::Fun,
    ];

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            NeedKind::Hunger => "hunger",
            NeedKind::Energy => "energy",
            NeedKind::Hygiene => "hygiene",
            NeedKind::Bladder => "bladder",
            NeedKind::Social => "social",
            NeedKind::Fun => "fun",
        }
    }
}

impl fmt::Display for NeedKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// How satisfied one need is, clamped to `0.000..=100.000`.
///
/// The invariant is maintained by construction: there is no way to build or
/// mutate a `NeedValue` that lands outside the range, so no system has to
/// remember to re-clamp after arithmetic.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(transparent)]
pub struct NeedValue(Milli);

impl NeedValue {
    /// The satisfied end of the scale, `100.000`.
    pub const FULL: Self = NeedValue(Milli(100_000));
    /// The critical end of the scale, `0.000`.
    pub const EMPTY: Self = NeedValue(Milli::ZERO);

    /// Clamps `value` into range.
    #[must_use]
    pub const fn new(value: Milli) -> Self {
        NeedValue(value.clamp_to(Milli::ZERO, Self::FULL.0))
    }

    #[must_use]
    pub const fn get(self) -> Milli {
        self.0
    }

    /// Reduces satisfaction, flooring at [`Self::EMPTY`].
    #[must_use]
    pub const fn drained_by(self, amount: Milli) -> Self {
        Self::new(self.0.saturating_sub(amount))
    }

    /// Increases satisfaction, capping at [`Self::FULL`].
    #[must_use]
    pub const fn restored_by(self, amount: Milli) -> Self {
        Self::new(self.0.saturating_add(amount))
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0.is_zero()
    }

    #[must_use]
    pub const fn is_full(self) -> bool {
        self.0.0 == Self::FULL.0.0
    }

    /// True when satisfaction has fallen to or below `threshold`.
    #[must_use]
    pub const fn is_at_or_below(self, threshold: Self) -> bool {
        self.0.0 <= threshold.0.0
    }

    /// Position on the scale as a fraction in thousandths: `Milli(500)` is
    /// halfway. Useful for weighting without reintroducing floats.
    #[must_use]
    pub const fn fraction(self) -> Milli {
        self.0.ratio_of(Self::FULL.0)
    }
}

impl fmt::Display for NeedValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

/// The six physical needs of one pet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhysicalNeeds {
    pub hunger: NeedValue,
    pub energy: NeedValue,
    pub hygiene: NeedValue,
    pub bladder: NeedValue,
    pub social: NeedValue,
    pub fun: NeedValue,
}

impl PhysicalNeeds {
    /// A freshly hatched pet: everything satisfied.
    #[must_use]
    pub const fn full() -> Self {
        PhysicalNeeds {
            hunger: NeedValue::FULL,
            energy: NeedValue::FULL,
            hygiene: NeedValue::FULL,
            bladder: NeedValue::FULL,
            social: NeedValue::FULL,
            fun: NeedValue::FULL,
        }
    }

    #[must_use]
    pub const fn get(&self, kind: NeedKind) -> NeedValue {
        match kind {
            NeedKind::Hunger => self.hunger,
            NeedKind::Energy => self.energy,
            NeedKind::Hygiene => self.hygiene,
            NeedKind::Bladder => self.bladder,
            NeedKind::Social => self.social,
            NeedKind::Fun => self.fun,
        }
    }

    pub const fn set(&mut self, kind: NeedKind, value: NeedValue) {
        match kind {
            NeedKind::Hunger => self.hunger = value,
            NeedKind::Energy => self.energy = value,
            NeedKind::Hygiene => self.hygiene = value,
            NeedKind::Bladder => self.bladder = value,
            NeedKind::Social => self.social = value,
            NeedKind::Fun => self.fun = value,
        }
    }

    /// Every need paired with its value, always in [`NeedKind::ALL`] order.
    pub fn iter(&self) -> impl Iterator<Item = (NeedKind, NeedValue)> + '_ {
        NeedKind::ALL.into_iter().map(|kind| (kind, self.get(kind)))
    }

    /// The least satisfied need.
    ///
    /// Ties break toward the earlier entry in [`NeedKind::ALL`]. The comparison
    /// is strict `<` for exactly that reason — `<=` would make the winner depend
    /// on iteration direction, which is the sort of detail that turns into a
    /// nondeterminism bug two phases later.
    #[must_use]
    pub fn lowest(&self) -> (NeedKind, NeedValue) {
        let mut lowest = (NeedKind::Hunger, self.hunger);
        for (kind, value) in self.iter() {
            if value < lowest.1 {
                lowest = (kind, value);
            }
        }
        lowest
    }

    /// True if any need has fallen to or below `threshold`.
    #[must_use]
    pub fn any_at_or_below(&self, threshold: NeedValue) -> bool {
        self.iter()
            .any(|(_, value)| value.is_at_or_below(threshold))
    }
}

impl Default for PhysicalNeeds {
    fn default() -> Self {
        Self::full()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construction_clamps_into_range() {
        assert_eq!(NeedValue::new(Milli(-1)), NeedValue::EMPTY);
        assert_eq!(NeedValue::new(Milli(999_999)), NeedValue::FULL);
        assert_eq!(NeedValue::new(Milli(50_000)).get(), Milli(50_000));
    }

    #[test]
    fn draining_floors_at_empty() {
        let need = NeedValue::new(Milli(1_000));
        assert_eq!(need.drained_by(Milli(500)).get(), Milli(500));
        assert_eq!(need.drained_by(Milli(5_000)), NeedValue::EMPTY);
        assert_eq!(
            NeedValue::EMPTY.drained_by(Milli(i32::MAX)),
            NeedValue::EMPTY
        );
    }

    #[test]
    fn restoring_caps_at_full() {
        let need = NeedValue::new(Milli(99_000));
        assert_eq!(need.restored_by(Milli(500)).get(), Milli(99_500));
        assert_eq!(need.restored_by(Milli(5_000)), NeedValue::FULL);
        assert_eq!(
            NeedValue::FULL.restored_by(Milli(i32::MAX)),
            NeedValue::FULL
        );
    }

    #[test]
    fn saturating_arithmetic_survives_extreme_inputs() {
        // A hostile save could carry an absurd modifier; it must clamp, not wrap.
        assert_eq!(NeedValue::FULL.drained_by(Milli(i32::MIN)), NeedValue::FULL);
        assert_eq!(
            NeedValue::EMPTY.restored_by(Milli(i32::MIN)),
            NeedValue::EMPTY
        );
    }

    #[test]
    fn fraction_reports_position_on_the_scale() {
        assert_eq!(NeedValue::FULL.fraction(), Milli::ONE);
        assert_eq!(NeedValue::EMPTY.fraction(), Milli::ZERO);
        assert_eq!(NeedValue::new(Milli(50_000)).fraction(), Milli(500));
    }

    #[test]
    fn threshold_comparison_is_inclusive() {
        let threshold = NeedValue::new(Milli(20_000));
        assert!(NeedValue::new(Milli(20_000)).is_at_or_below(threshold));
        assert!(NeedValue::new(Milli(19_999)).is_at_or_below(threshold));
        assert!(!NeedValue::new(Milli(20_001)).is_at_or_below(threshold));
    }

    #[test]
    fn a_new_pet_starts_satisfied() {
        let needs = PhysicalNeeds::full();
        assert!(needs.iter().all(|(_, value)| value.is_full()));
        assert_eq!(PhysicalNeeds::default(), needs);
    }

    #[test]
    fn every_need_is_reachable_by_kind() {
        let mut needs = PhysicalNeeds::full();
        for (index, kind) in NeedKind::ALL.into_iter().enumerate() {
            let value = NeedValue::new(Milli(index as i32 * 1_000));
            needs.set(kind, value);
            assert_eq!(needs.get(kind), value, "{kind} did not round-trip");
        }
        // Every kind addressed a distinct field; nothing was aliased.
        let values: Vec<_> = needs.iter().map(|(_, value)| value).collect();
        let mut unique = values.clone();
        unique.dedup();
        assert_eq!(values.len(), unique.len());
    }

    #[test]
    fn iteration_order_is_fixed() {
        let kinds: Vec<_> = PhysicalNeeds::full().iter().map(|(kind, _)| kind).collect();
        assert_eq!(kinds, NeedKind::ALL.to_vec());
    }

    #[test]
    fn lowest_finds_the_least_satisfied_need() {
        let mut needs = PhysicalNeeds::full();
        needs.set(NeedKind::Fun, NeedValue::new(Milli(10_000)));
        needs.set(NeedKind::Social, NeedValue::new(Milli(30_000)));
        assert_eq!(
            needs.lowest(),
            (NeedKind::Fun, NeedValue::new(Milli(10_000)))
        );
    }

    #[test]
    fn lowest_breaks_ties_toward_the_first_declared_need() {
        // All equal: the answer must be Hunger, deterministically, every time.
        let needs = PhysicalNeeds::full();
        assert_eq!(needs.lowest().0, NeedKind::Hunger);

        let mut tied = PhysicalNeeds::full();
        tied.set(NeedKind::Energy, NeedValue::EMPTY);
        tied.set(NeedKind::Fun, NeedValue::EMPTY);
        assert_eq!(
            tied.lowest().0,
            NeedKind::Energy,
            "the earlier kind must win a tie"
        );
    }

    #[test]
    fn critical_detection_scans_every_need() {
        let threshold = NeedValue::new(Milli(15_000));
        assert!(!PhysicalNeeds::full().any_at_or_below(threshold));

        for kind in NeedKind::ALL {
            let mut needs = PhysicalNeeds::full();
            needs.set(kind, NeedValue::new(Milli(1_000)));
            assert!(needs.any_at_or_below(threshold), "{kind} was not detected");
        }
    }

    #[test]
    fn needs_survive_a_serde_round_trip() {
        let mut needs = PhysicalNeeds::full();
        needs.set(NeedKind::Hunger, NeedValue::new(Milli(42_000)));
        let encoded = serde_json::to_string(&needs).expect("serialize");
        let decoded: PhysicalNeeds = serde_json::from_str(&encoded).expect("deserialize");
        assert_eq!(needs, decoded);
    }
}
