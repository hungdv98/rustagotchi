//! Game time. See ADR 0003.
//!
//! The simulation advances only in whole [`Tick`]s, one tick per game-minute,
//! and one game-minute per real minute. Variable timesteps do not exist, which
//! is what allows offline catch-up to be the same code as the live loop rather
//! than an approximation of it.
//!
//! Wall-clock time appears here only as [`UnixSeconds`], a plain value handed in
//! across an `application` port. This crate cannot read a clock — `SystemTime`
//! and `Instant` are rejected by Clippy.

pub mod duration;
pub mod game_time;
pub mod timestamp;

pub use duration::GameDuration;
pub use game_time::{
    GameClock, MAX_OFFLINE_CATCHUP_TICKS, SECONDS_PER_TICK, TICKS_PER_DAY, TICKS_PER_HOUR,
    TICKS_PER_YEAR, Tick,
};
pub use timestamp::UnixSeconds;
