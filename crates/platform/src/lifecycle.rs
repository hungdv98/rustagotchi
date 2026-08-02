use std::fmt;
use thiserror::Error;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LifecycleState {
    #[default]
    Starting,
    Running,
    Suspended,
    Exiting,
    Exited,
}

impl fmt::Display for LifecycleState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", format!("{self:?}").to_lowercase())
    }
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
#[error("invalid lifecycle transition from {from} to {to}")]
pub struct LifecycleError {
    pub from: LifecycleState,
    pub to: LifecycleState,
}

#[derive(Debug, Default)]
pub struct Lifecycle {
    state: LifecycleState,
}

impl Lifecycle {
    #[must_use]
    pub fn state(&self) -> LifecycleState {
        self.state
    }

    pub fn mark_running(&mut self) -> Result<(), LifecycleError> {
        self.transition(LifecycleState::Running, &[LifecycleState::Starting])
    }

    pub fn suspend(&mut self) -> Result<(), LifecycleError> {
        self.transition(LifecycleState::Suspended, &[LifecycleState::Running])
    }

    pub fn resume(&mut self) -> Result<(), LifecycleError> {
        self.transition(LifecycleState::Running, &[LifecycleState::Suspended])
    }

    pub fn begin_exit(&mut self) -> Result<(), LifecycleError> {
        self.transition(
            LifecycleState::Exiting,
            &[
                LifecycleState::Starting,
                LifecycleState::Running,
                LifecycleState::Suspended,
            ],
        )
    }

    pub fn finish_exit(&mut self) -> Result<(), LifecycleError> {
        self.transition(LifecycleState::Exited, &[LifecycleState::Exiting])
    }

    fn transition(
        &mut self,
        next: LifecycleState,
        allowed_from: &[LifecycleState],
    ) -> Result<(), LifecycleError> {
        if !allowed_from.contains(&self.state) {
            return Err(LifecycleError {
                from: self.state,
                to: next,
            });
        }
        self.state = next;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supports_normal_desktop_lifecycle() {
        let mut lifecycle = Lifecycle::default();
        lifecycle.mark_running().unwrap();
        lifecycle.suspend().unwrap();
        lifecycle.resume().unwrap();
        lifecycle.begin_exit().unwrap();
        lifecycle.finish_exit().unwrap();
        assert_eq!(lifecycle.state(), LifecycleState::Exited);
    }

    #[test]
    fn rejects_invalid_transitions() {
        let mut lifecycle = Lifecycle::default();
        let error = lifecycle.resume().unwrap_err();
        assert_eq!(error.from, LifecycleState::Starting);
        assert_eq!(error.to, LifecycleState::Running);
    }
}
