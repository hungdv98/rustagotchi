//! Pure business rules for Rustagotchi.
//!
//! This crate is the bottom of the dependency graph and depends on nothing that
//! touches the outside world: no Bevy, no SQLite, no filesystem, no OS APIs, no
//! UI, no application configuration. That constraint is enforced mechanically by
//! `scripts/check-deps.*` (dependency snapshot) and `crates/domain/clippy.toml`
//! (banned types), not by convention.
//!
//! Two rules govern everything written here:
//!
//! * **Integer math only.** Simulated quantities are [`units::Milli`] or
//!   [`units::Fixed`]; `f32`/`f64` are banned. Exact equality is what makes the
//!   offline/live and save/load equivalence tests possible. See ADR 0006.
//! * **No ambient randomness.** RNG is derived as a pure function of
//!   `(world seed, stream, tick, entity)` via [`rng`], never held as mutable
//!   state. See ADR 0007.
//!
//! # Module layout
//!
//! Landed: [`units`], [`rng`], [`identifiers`], [`time`], [`error`],
//! [`validation`], [`events`], and the [`pet`] life stage and needs.
//!
//! Still to come — Phase 1 remainder: the rest of [`pet`] (stats, emotions,
//! personality, health, progression), `genetics`, `inventory`, `economy`.
//! Phase 5: `breeding`, `combat`. Phase 7: `quests`, `achievements`, `world`.

#![doc(html_no_source)]

pub mod breeding;
pub mod combat;
pub mod economy;
pub mod error;
pub mod events;
pub mod genetics;
pub mod identifiers;
pub mod inventory;
pub mod pet;
pub mod rng;
pub mod time;
pub mod units;
pub mod validation;
