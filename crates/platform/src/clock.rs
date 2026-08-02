use application::ports::Clock;
use domain::time::UnixSeconds;

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_unix(&self) -> UnixSeconds {
        UnixSeconds(chrono::Utc::now().timestamp())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_clock_returns_a_plausible_timestamp() {
        assert!(SystemClock.now_unix().0 > 1_700_000_000);
    }
}
