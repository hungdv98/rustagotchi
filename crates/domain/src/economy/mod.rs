//! Currency, the wallet, and the record of money moving.
//!
//! Prices, shop stock and reward tables are content and balance data
//! (ADR 0005); this module owns only the rules that must always hold — a
//! balance cannot go negative, and a transaction's direction follows from its
//! reason rather than from a sign someone remembered to set.
//!
//! `shop` and `pricing` arrive in Phase 7 with the shop screen; both need the
//! content registry to mean anything.

pub mod transaction;
pub mod wallet;

pub use transaction::{Transaction, TransactionReason};
pub use wallet::{Currency, Wallet};
