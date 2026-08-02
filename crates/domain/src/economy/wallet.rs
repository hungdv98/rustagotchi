//! Currency and the player's balance.

use core::fmt;

use serde::{Deserialize, Serialize};

use crate::error::DomainError;

/// Soft currency, in whole coins.
///
/// Integer, like everything else the simulation touches (ADR 0006) — a currency
/// stored as a float is the classic way to make two players' totals disagree
/// after the same sequence of purchases.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Currency(pub i64);

impl Currency {
    pub const ZERO: Self = Currency(0);

    #[must_use]
    pub const fn saturating_add(self, rhs: Self) -> Self {
        Currency(self.0.saturating_add(rhs.0))
    }

    #[must_use]
    pub const fn saturating_sub(self, rhs: Self) -> Self {
        Currency(self.0.saturating_sub(rhs.0))
    }

    #[must_use]
    pub const fn is_positive(self) -> bool {
        self.0 > 0
    }
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}c", self.0)
    }
}

/// The player's balance.
///
/// Cannot go negative. There is no credit in this game, so an overdraft would
/// only ever be a bug — and one that would be discovered by a player finding
/// themselves unable to buy anything for an hour.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Wallet {
    balance: Currency,
}

impl Wallet {
    #[must_use]
    pub const fn with_balance(balance: Currency) -> Self {
        Wallet { balance }
    }

    #[must_use]
    pub const fn balance(&self) -> Currency {
        self.balance
    }

    #[must_use]
    pub const fn can_afford(&self, amount: Currency) -> bool {
        self.balance.0 >= amount.0
    }

    /// Adds `amount`. Negative amounts are rejected — taking money away is
    /// [`Self::withdraw`], which is the operation that can fail.
    ///
    /// # Errors
    ///
    /// [`DomainError::NotAllowed`] if `amount` is negative.
    pub fn deposit(&mut self, amount: Currency) -> Result<(), DomainError> {
        if amount.0 < 0 {
            return Err(DomainError::NotAllowed {
                action: "deposit",
                reason: "amount is negative; use withdraw instead",
            });
        }
        self.balance = self.balance.saturating_add(amount);
        Ok(())
    }

    /// Removes `amount`.
    ///
    /// # Errors
    ///
    /// [`DomainError::Insufficient`] if the balance is too low, or
    /// [`DomainError::NotAllowed`] if `amount` is negative. Either way the
    /// balance is untouched.
    pub fn withdraw(&mut self, amount: Currency) -> Result<(), DomainError> {
        if amount.0 < 0 {
            return Err(DomainError::NotAllowed {
                action: "withdraw",
                reason: "amount is negative; use deposit instead",
            });
        }
        if !self.can_afford(amount) {
            return Err(DomainError::Insufficient {
                resource: "currency",
                needed: amount.0,
                available: self.balance.0,
            });
        }
        self.balance = self.balance.saturating_sub(amount);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deposits_and_withdrawals_move_the_balance() {
        let mut wallet = Wallet::with_balance(Currency(100));
        wallet.deposit(Currency(50)).expect("deposit");
        assert_eq!(wallet.balance(), Currency(150));
        wallet.withdraw(Currency(30)).expect("withdraw");
        assert_eq!(wallet.balance(), Currency(120));
    }

    #[test]
    fn a_wallet_cannot_go_negative() {
        let mut wallet = Wallet::with_balance(Currency(10));
        let error = wallet.withdraw(Currency(11)).unwrap_err();
        assert!(matches!(error, DomainError::Insufficient { .. }));
        assert_eq!(
            wallet.balance(),
            Currency(10),
            "a failed withdrawal must change nothing"
        );
    }

    #[test]
    fn spending_the_exact_balance_is_allowed() {
        let mut wallet = Wallet::with_balance(Currency(10));
        wallet.withdraw(Currency(10)).expect("exact spend");
        assert_eq!(wallet.balance(), Currency::ZERO);
    }

    #[test]
    fn negative_amounts_are_refused_in_both_directions() {
        let mut wallet = Wallet::with_balance(Currency(10));
        assert!(
            wallet.deposit(Currency(-5)).is_err(),
            "a negative deposit is a disguised theft"
        );
        assert!(
            wallet.withdraw(Currency(-5)).is_err(),
            "a negative withdrawal is free money"
        );
        assert_eq!(wallet.balance(), Currency(10));
    }

    #[test]
    fn affordability_matches_what_withdrawal_allows() {
        let wallet = Wallet::with_balance(Currency(10));
        assert!(wallet.can_afford(Currency(10)));
        assert!(!wallet.can_afford(Currency(11)));
    }

    #[test]
    fn a_huge_deposit_saturates_instead_of_wrapping() {
        let mut wallet = Wallet::with_balance(Currency(i64::MAX));
        wallet.deposit(Currency(1_000)).expect("deposit");
        assert_eq!(wallet.balance(), Currency(i64::MAX));
    }
}
