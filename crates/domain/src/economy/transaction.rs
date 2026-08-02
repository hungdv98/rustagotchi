//! A record of money moving.

use serde::{Deserialize, Serialize};

use super::wallet::Currency;
use crate::time::Tick;

/// Why money moved.
///
/// Coarse categories, not free text: this feeds the economy balancing report,
/// and a category someone typed by hand cannot be aggregated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum TransactionReason {
    BattleReward,
    QuestReward,
    MinigameReward,
    ItemSold,
    ItemBought,
    BreedingFee,
    MedicalFee,
}

impl TransactionReason {
    /// Whether this reason adds money or removes it.
    ///
    /// Derived from the reason rather than stored alongside it, so a record
    /// cannot claim to be a `BattleReward` that took money away.
    #[must_use]
    pub const fn is_income(self) -> bool {
        matches!(
            self,
            TransactionReason::BattleReward
                | TransactionReason::QuestReward
                | TransactionReason::MinigameReward
                | TransactionReason::ItemSold
        )
    }
}

/// One movement of currency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transaction {
    pub reason: TransactionReason,
    /// Always non-negative; direction comes from
    /// [`TransactionReason::is_income`].
    pub amount: Currency,
    pub at: Tick,
}

impl Transaction {
    /// Builds a record, normalising the amount to a magnitude.
    #[must_use]
    pub fn new(reason: TransactionReason, amount: Currency, at: Tick) -> Self {
        Transaction {
            reason,
            amount: Currency(amount.0.saturating_abs()),
            at,
        }
    }

    /// The signed effect on a balance.
    #[must_use]
    pub const fn signed_amount(&self) -> Currency {
        if self.reason.is_income() {
            self.amount
        } else {
            Currency(-self.amount.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn income_and_expense_reasons_are_classified() {
        assert!(TransactionReason::BattleReward.is_income());
        assert!(TransactionReason::ItemSold.is_income());
        assert!(!TransactionReason::ItemBought.is_income());
        assert!(!TransactionReason::BreedingFee.is_income());
    }

    #[test]
    fn amounts_are_stored_as_magnitudes() {
        let record = Transaction::new(TransactionReason::ItemBought, Currency(-50), Tick(1));
        assert_eq!(
            record.amount,
            Currency(50),
            "direction comes from the reason, not the sign"
        );
    }

    #[test]
    fn the_signed_amount_follows_the_reason() {
        let earned = Transaction::new(TransactionReason::QuestReward, Currency(30), Tick(1));
        let spent = Transaction::new(TransactionReason::MedicalFee, Currency(30), Tick(1));
        assert_eq!(earned.signed_amount(), Currency(30));
        assert_eq!(spent.signed_amount(), Currency(-30));
    }

    #[test]
    fn a_ledger_sums_to_the_expected_balance() {
        let ledger = [
            Transaction::new(TransactionReason::BattleReward, Currency(100), Tick(1)),
            Transaction::new(TransactionReason::ItemBought, Currency(40), Tick(2)),
            Transaction::new(TransactionReason::QuestReward, Currency(25), Tick(3)),
        ];
        let total: i64 = ledger.iter().map(|entry| entry.signed_amount().0).sum();
        assert_eq!(total, 85);
    }
}
