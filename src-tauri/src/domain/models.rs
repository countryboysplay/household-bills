use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use super::money::Money;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaycheckAmount {
    pub projected: Money,
    pub expected: Option<Money>,
    pub actual: Option<Money>,
}

impl PaycheckAmount {
    /// Actual always wins, followed by a manually entered expectation, followed
    /// by the recurring projected amount.
    pub fn effective(&self) -> Money {
        self.actual.or(self.expected).unwrap_or(self.projected)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PaycheckForSchedule {
    pub id: String,
    pub pay_date: NaiveDate,
    pub amount: Money,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PaymentType {
    Manual,
    Autopay,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Flexible,
    Normal,
    Essential,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BillForSchedule {
    pub id: String,
    pub due_date: NaiveDate,
    pub earliest_payment_date: NaiveDate,
    pub latest_payment_date: NaiveDate,
    pub amount: Money,
    pub payment_type: PaymentType,
    pub priority: Priority,
    pub locked_paycheck_id: Option<String>,
    /// The existing assignment is a stability preference, not a hard lock.
    pub existing_paycheck_id: Option<String>,
    pub can_split: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FundingSourceType {
    CurrentCash,
    Paycheck,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleReasonCode {
    CurrentCash,
    LatestEligiblePaycheck,
    AutopayLatestEligiblePaycheck,
    StableExistingAssignment,
    MovedEarlierToProtectBuffer,
    UserLock,
    SplitAcrossPaychecks,
    ReservedAcrossPaychecks,
    AllocatedWithShortage,
    PartialFunding,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BillAllocationResult {
    pub bill_id: String,
    pub paycheck_id: Option<String>,
    pub funding_source_type: FundingSourceType,
    pub amount: Money,
    /// For a manual bill this is when the app recommends making the payment.
    /// For autopay this remains the fixed draft/due date regardless of which
    /// paycheck funds it.
    pub payment_date: NaiveDate,
    pub reason_code: ScheduleReasonCode,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum OptionalCommitmentKind {
    ExtraDebt,
    OptionalSavings,
    SinkingFund,
}

impl OptionalCommitmentKind {
    /// Higher values are preserved longer when a paycheck is tight.
    pub const fn preservation_rank(self) -> u8 {
        match self {
            Self::ExtraDebt => 10,
            Self::OptionalSavings => 20,
            Self::SinkingFund => 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OptionalCommitmentForSchedule {
    pub id: String,
    pub scheduled_date: NaiveDate,
    pub target_paycheck_id: Option<String>,
    pub amount: Money,
    pub minimum_amount: Money,
    pub kind: OptionalCommitmentKind,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paycheck_amount_precedence_is_actual_expected_projected() {
        let mut p = PaycheckAmount {
            projected: Money::cents(217_500),
            expected: None,
            actual: None,
        };
        assert_eq!(p.effective(), Money::cents(217_500));
        p.expected = Some(Money::cents(234_718));
        assert_eq!(p.effective(), Money::cents(234_718));
        p.actual = Some(Money::cents(233_142));
        assert_eq!(p.effective(), Money::cents(233_142));
    }
}
