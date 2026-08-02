//! What a pet remembers.

use core::fmt;

use serde::{Deserialize, Serialize};

use crate::time::Tick;
use crate::units::Milli;

/// A kind of remembered event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum MemoryKind {
    Fed,
    Played,
    Groomed,
    Healed,
    Neglected,
    Scolded,
    WonBattle,
    LostBattle,
}

impl MemoryKind {
    /// Whether the memory is a fond one.
    #[must_use]
    pub const fn is_pleasant(self) -> bool {
        matches!(
            self,
            MemoryKind::Fed
                | MemoryKind::Played
                | MemoryKind::Groomed
                | MemoryKind::Healed
                | MemoryKind::WonBattle
        )
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            MemoryKind::Fed => "fed",
            MemoryKind::Played => "played",
            MemoryKind::Groomed => "groomed",
            MemoryKind::Healed => "healed",
            MemoryKind::Neglected => "neglected",
            MemoryKind::Scolded => "scolded",
            MemoryKind::WonBattle => "won a battle",
            MemoryKind::LostBattle => "lost a battle",
        }
    }
}

impl fmt::Display for MemoryKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// One remembered event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Memory {
    pub kind: MemoryKind,
    pub at: Tick,
    pub strength: Milli,
}

/// A pet's bounded memory.
///
/// Capacity is fixed. A pet simulated for a year receives hundreds of thousands
/// of ticks; an unbounded memory would make the save grow without limit and turn
/// every "what does it remember" query into a linear scan of history.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Memories {
    entries: Vec<Memory>,
}

impl Memories {
    pub const CAPACITY: usize = 32;
    pub const MAX_STRENGTH: Milli = Milli(100_000);
    /// Below this a memory is forgotten outright.
    pub const FORGET_BELOW: Milli = Milli(500);

    #[must_use]
    pub fn new() -> Self {
        Memories::default()
    }

    /// Records a memory, evicting the weakest if full.
    ///
    /// Eviction is by strength, then by age — weakest first, and among equals
    /// the oldest. Both tie-breaks are needed: without the second, which memory
    /// gets dropped would depend on vector order, which is exactly the sort of
    /// incidental detail that diverges between a live run and a replay.
    ///
    /// Returns the memory that was displaced, if any.
    pub fn record(&mut self, kind: MemoryKind, at: Tick, strength: Milli) -> Option<Memory> {
        let strength = strength.clamp_to(Milli::ZERO, Self::MAX_STRENGTH);
        if strength < Self::FORGET_BELOW {
            return None;
        }

        let displaced = if self.entries.len() >= Self::CAPACITY {
            let weakest = self
                .entries
                .iter()
                .enumerate()
                .min_by_key(|(_, memory)| (memory.strength, memory.at))
                .map(|(index, _)| index);

            match weakest {
                // Nothing is displaced if the newcomer is not worth keeping.
                Some(index)
                    if (self.entries[index].strength, self.entries[index].at) < (strength, at) =>
                {
                    Some(self.entries.remove(index))
                }
                _ => return None,
            }
        } else {
            None
        };

        self.entries.push(Memory { kind, at, strength });
        displaced
    }

    /// Weakens every memory, forgetting those that fall below the floor.
    pub fn decay(&mut self, amount: Milli) {
        if amount.0 <= 0 {
            return;
        }
        self.entries.retain_mut(|memory| {
            memory.strength = memory.strength.saturating_sub(amount);
            memory.strength >= Self::FORGET_BELOW
        });
    }

    #[must_use]
    pub fn count(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[must_use]
    pub fn count_of(&self, kind: MemoryKind) -> usize {
        self.entries
            .iter()
            .filter(|memory| memory.kind == kind)
            .count()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Memory> {
        self.entries.iter()
    }

    /// Net sentiment: pleasant memories minus unpleasant ones, by strength.
    #[must_use]
    pub fn sentiment(&self) -> Milli {
        self.entries.iter().fold(Milli::ZERO, |total, memory| {
            if memory.kind.is_pleasant() {
                total.saturating_add(memory.strength)
            } else {
                total.saturating_sub(memory.strength)
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_pet_remembers_nothing() {
        let memories = Memories::new();
        assert!(memories.is_empty());
        assert_eq!(memories.sentiment(), Milli::ZERO);
    }

    #[test]
    fn recording_stores_a_memory() {
        let mut memories = Memories::new();
        memories.record(MemoryKind::Fed, Tick(1), Milli(50_000));
        assert_eq!(memories.count(), 1);
        assert_eq!(memories.count_of(MemoryKind::Fed), 1);
        assert_eq!(memories.count_of(MemoryKind::Scolded), 0);
    }

    #[test]
    fn a_memory_too_faint_to_matter_is_not_recorded() {
        let mut memories = Memories::new();
        assert!(
            memories
                .record(MemoryKind::Fed, Tick(1), Milli(1))
                .is_none()
        );
        assert!(memories.is_empty());
    }

    #[test]
    fn strength_is_capped() {
        let mut memories = Memories::new();
        memories.record(MemoryKind::Fed, Tick(1), Milli(i32::MAX));
        assert_eq!(
            memories.iter().next().expect("recorded").strength,
            Memories::MAX_STRENGTH
        );
    }

    #[test]
    fn capacity_is_bounded_and_the_weakest_is_evicted() {
        let mut memories = Memories::new();
        for index in 0..Memories::CAPACITY {
            memories.record(MemoryKind::Played, Tick(index as u64), Milli(10_000));
        }
        assert_eq!(memories.count(), Memories::CAPACITY);

        let displaced = memories.record(MemoryKind::WonBattle, Tick(999), Milli(90_000));
        assert!(
            displaced.is_some(),
            "a stronger memory should displace a weaker one"
        );
        assert_eq!(
            memories.count(),
            Memories::CAPACITY,
            "capacity must not grow"
        );
        assert_eq!(memories.count_of(MemoryKind::WonBattle), 1);
    }

    #[test]
    fn a_weaker_memory_does_not_displace_a_stronger_one() {
        let mut memories = Memories::new();
        for index in 0..Memories::CAPACITY {
            memories.record(MemoryKind::Played, Tick(index as u64), Milli(90_000));
        }
        let displaced = memories.record(MemoryKind::Fed, Tick(999), Milli(1_000));
        assert!(
            displaced.is_none(),
            "a faint memory must not evict a vivid one"
        );
        assert_eq!(memories.count_of(MemoryKind::Fed), 0);
    }

    #[test]
    fn eviction_prefers_the_oldest_among_equally_weak_memories() {
        let mut memories = Memories::new();
        for index in 0..Memories::CAPACITY {
            memories.record(MemoryKind::Played, Tick(100 + index as u64), Milli(10_000));
        }
        let displaced = memories
            .record(MemoryKind::Fed, Tick(999), Milli(10_001))
            .expect("evicted");
        assert_eq!(
            displaced.at,
            Tick(100),
            "the oldest equally-weak memory goes first"
        );
    }

    #[test]
    fn decay_weakens_and_eventually_forgets() {
        let mut memories = Memories::new();
        memories.record(MemoryKind::Fed, Tick(1), Milli(2_000));
        memories.record(MemoryKind::Played, Tick(2), Milli(50_000));

        memories.decay(Milli(1_000));
        assert_eq!(memories.count(), 2);

        memories.decay(Milli(1_000));
        assert_eq!(memories.count(), 1, "the faded memory is dropped");
        assert_eq!(memories.count_of(MemoryKind::Played), 1);
    }

    #[test]
    fn decay_never_leaves_a_negative_strength() {
        let mut memories = Memories::new();
        memories.record(MemoryKind::Fed, Tick(1), Milli(50_000));
        memories.decay(Milli(i32::MAX));
        assert!(memories.is_empty());
    }

    #[test]
    fn sentiment_weighs_pleasant_against_unpleasant() {
        let mut memories = Memories::new();
        memories.record(MemoryKind::Fed, Tick(1), Milli(30_000));
        memories.record(MemoryKind::Neglected, Tick(2), Milli(10_000));
        assert_eq!(memories.sentiment(), Milli(20_000));

        memories.record(MemoryKind::Scolded, Tick(3), Milli(40_000));
        assert_eq!(memories.sentiment(), Milli(-20_000));
    }
}
