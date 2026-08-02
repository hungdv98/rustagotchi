//! Minigames: rules and state only, no rendering.
//!
//! Each game is a headless state machine that ends in a `MinigameResult`. The
//! domain converts that result into need and currency changes -- minigames
//! never mutate a `Pet` directly, so a new game can be added without touching
//! gameplay balance, and every game is testable without a window.
//!
//! Phase 7 builds: `minigame`, `registry`, `result`, `memory_match/*`,
//! `obstacle_run/*`, `food_catch/*`.
