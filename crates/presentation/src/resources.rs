use application::GameEngine;
use application::ports::SaveRepository;
use bevy::prelude::*;
use domain::identifiers::SaveSlotId;

pub(crate) type DesktopGameEngine = GameEngine<Box<dyn SaveRepository>>;

#[derive(Resource)]
pub struct EngineResource(pub DesktopGameEngine);

impl std::fmt::Debug for EngineResource {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_tuple("EngineResource")
            .field(&self.0)
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Resource)]
pub struct ActiveSaveSlot(pub SaveSlotId);

#[derive(Clone, Copy, Debug, Resource)]
pub struct PetSelection {
    pub primary: usize,
    pub secondary: usize,
}

impl Default for PetSelection {
    fn default() -> Self {
        Self {
            primary: 0,
            secondary: 1,
        }
    }
}

impl PetSelection {
    #[must_use]
    pub fn primary(self, pet_count: usize) -> Option<usize> {
        (pet_count > 0).then(|| self.primary % pet_count)
    }

    #[must_use]
    pub fn pair(self, pet_count: usize) -> Option<[usize; 2]> {
        if pet_count < 2 {
            return None;
        }
        let primary = self.primary % pet_count;
        let mut secondary = self.secondary % pet_count;
        if secondary == primary {
            secondary = (primary + 1) % pet_count;
        }
        Some([primary, secondary])
    }

    pub fn cycle_primary(&mut self, pet_count: usize) {
        if pet_count == 0 {
            return;
        }
        self.primary = (self.primary + 1) % pet_count;
    }

    pub fn cycle_secondary(&mut self, pet_count: usize) {
        if pet_count < 2 {
            return;
        }
        self.secondary = (self.secondary + 1) % pet_count;
        if self.secondary == self.primary % pet_count {
            self.secondary = (self.secondary + 1) % pet_count;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_always_returns_a_distinct_in_range_pair() {
        let mut selection = PetSelection::default();
        for _ in 0..12 {
            let [first, second] = selection.pair(3).unwrap();
            assert!(first < 3 && second < 3);
            assert_ne!(first, second);
            selection.cycle_primary(3);
            selection.cycle_secondary(3);
        }
        assert_eq!(selection.pair(1), None);
        assert_eq!(selection.primary(0), None);
        assert_eq!(
            PetSelection {
                primary: 7,
                secondary: 0
            }
            .primary(3),
            Some(1)
        );
    }

    #[test]
    fn primary_cycles_between_exactly_two_pets() {
        let mut selection = PetSelection::default();

        selection.cycle_primary(2);
        assert_eq!(selection.primary(2), Some(1));
        assert_eq!(selection.pair(2), Some([1, 0]));

        selection.cycle_primary(2);
        assert_eq!(selection.primary(2), Some(0));
        assert_eq!(selection.pair(2), Some([0, 1]));
    }
}
