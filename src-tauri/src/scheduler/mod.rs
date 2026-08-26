pub mod recurrence;

use std::cmp::Reverse;
use std::collections::{BTreeMap, HashMap};

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::domain::{
    money::Money,
    models::{
        BillAllocationResult, BillForSchedule, FundingSourceType,
        OptionalCommitmentForSchedule, OptionalCommitmentKind, PaycheckForSchedule,
        PaymentType, ScheduleReasonCode,
    },
};

const CURRENT_CASH_BUCKET_ID: &str = "__current_cash__";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleInput {
    pub planning_date: NaiveDate,
    pub starting_cash: Money,
    pub protected_buffer: Money,
    pub tight_headroom: Money,
    pub paychecks: Vec<PaycheckForSchedule>,
    pub bills: Vec<BillForSchedule>,
    pub optional_commitments: Vec<OptionalCommitmentForSchedule>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BucketStatus {
    Healthy,
    Tight,
    Shortage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FundingBucketSummary {
    pub bucket_id: String,
    pub funding_source_type: FundingSourceType,
    pub date: NaiveDate,
    pub gross_amount: Money,
    pub buffer_replenishment: Money,
    pub allocatable_capacity: Money,
    pub bill_allocations: Money,
    pub optional_commitments: Money,
    pub remaining_headroom: Money,
    pub status: BucketStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WarningCode {
    NoEligibleFundingSource,
    InvalidLockedPaycheck,
    BillPastDue,
    FundingShortage,
    PartialFunding,
    ProjectedBelowBuffer,
    ProjectedNegativeBalance,
    OptionalCommitmentReduced,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScheduleWarning {
    pub code: WarningCode,
    pub entity_id: Option<String>,
    pub date: Option<NaiveDate>,
    pub amount: Option<Money>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionEventType {
    StartingBalance,
    Paycheck,
    BillPayment,
    OptionalCommitment,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectionEvent {
    pub date: NaiveDate,
    pub event_type: ProjectionEventType,
    pub entity_id: Option<String>,
    pub amount: Money,
    pub balance_after: Money,
    pub safe_to_spend_after: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommitmentAdjustment {
    pub commitment_id: String,
    pub kind: OptionalCommitmentKind,
    pub requested_amount: Money,
    pub effective_amount: Money,
    pub reduced_by: Money,
    pub funding_bucket_id: Option<String>,
    pub scheduled_date: NaiveDate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleResult {
    pub allocations: Vec<BillAllocationResult>,
    pub unresolved_bill_ids: Vec<String>,
    pub bucket_summaries: Vec<FundingBucketSummary>,
    pub commitment_adjustments: Vec<CommitmentAdjustment>,
    pub projection: Vec<ProjectionEvent>,
    pub warnings: Vec<ScheduleWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaycheckChangeSimulation {
    pub paycheck_id: String,
    pub old_amount: Money,
    pub new_amount: Money,
    pub baseline: ScheduleResult,
    pub scenario: ScheduleResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualMoveSimulation {
    pub bill_id: String,
    pub source_bucket_id: Option<String>,
    pub target_paycheck_id: String,
    pub valid: bool,
    pub source_headroom_before: Option<Money>,
    pub source_headroom_after: Option<Money>,
    pub target_headroom_before: Option<Money>,
    pub target_headroom_after: Option<Money>,
    pub scenario_warnings: Vec<ScheduleWarning>,
}

#[derive(Debug, Clone)]
struct FundingBucket {
    id: String,
    source_type: FundingSourceType,
    date: NaiveDate,
    gross_amount: Money,
    buffer_replenishment: Money,
    capacity: Money,
    bill_allocations: Money,
    optional_commitments: Money,
}

impl FundingBucket {
    fn remaining(&self) -> Money {
        self.capacity - self.bill_allocations - self.optional_commitments
    }

    fn consume_bill(&mut self, amount: Money) {
        self.bill_allocations += amount;
    }

    fn consume_optional(&mut self, amount: Money) {
        self.optional_commitments += amount;
    }
}

pub fn safe_to_spend(balance: Money, protected_buffer: Money) -> Money {
    Money::cents((balance.value() - protected_buffer.value()).max(0))
}

fn format_money(amount: Money) -> String {
    let cents = amount.value().abs();
    format!("${}.{:02}", cents / 100, cents % 100)
}

/// Average of the supplied payment history, rounded half-up to the nearest cent.
///
/// Callers choose the window (the database applies each template's
/// `estimate_window_count`); this averages exactly what it is given. It is the one
/// implementation of the estimate — truncating a second copy elsewhere would drift
/// a cent away from what the tests assert.
pub fn estimate_variable_bill(history: &[Money], fallback: Money) -> Money {
    if history.is_empty() {
        return fallback;
    }
    let values = history;
    let total: i64 = values.iter().map(|m| m.value()).sum();
    let divisor = values.len() as i64;
    let rounded = if total >= 0 {
        (total + divisor / 2) / divisor
    } else {
        (total - divisor / 2) / divisor
    };
    Money::cents(rounded)
}

pub fn latest_eligible_paycheck<'a>(
    bill: &BillForSchedule,
    paychecks: &'a [PaycheckForSchedule],
) -> Option<&'a PaycheckForSchedule> {
    paychecks
        .iter()
        .filter(|p| p.pay_date >= bill.earliest_payment_date && p.pay_date <= bill.latest_payment_date)
        .max_by_key(|p| p.pay_date)
}

fn build_funding_buckets(input: &ScheduleInput) -> Vec<FundingBucket> {
    let starting_cash = input.starting_cash.max(Money::ZERO);
    let protected_buffer = input.protected_buffer.max(Money::ZERO);
    let protected_from_start = starting_cash.min(protected_buffer);
    let mut buffer_deficit = (protected_buffer - starting_cash).max(Money::ZERO);

    let mut buckets = vec![FundingBucket {
        id: CURRENT_CASH_BUCKET_ID.into(),
        source_type: FundingSourceType::CurrentCash,
        date: input.planning_date,
        gross_amount: starting_cash,
        buffer_replenishment: protected_from_start,
        capacity: (starting_cash - protected_buffer).max(Money::ZERO),
        bill_allocations: Money::ZERO,
        optional_commitments: Money::ZERO,
    }];

    let mut paychecks = input.paychecks.clone();
    paychecks.sort_by_key(|p| (p.pay_date, p.id.clone()));
    for paycheck in paychecks {
        let paycheck_amount = paycheck.amount.max(Money::ZERO);
        let replenish = paycheck_amount.min(buffer_deficit);
        buffer_deficit -= replenish;
        buckets.push(FundingBucket {
            id: paycheck.id,
            source_type: FundingSourceType::Paycheck,
            date: paycheck.pay_date,
            gross_amount: paycheck_amount,
            buffer_replenishment: replenish,
            capacity: paycheck_amount - replenish,
            bill_allocations: Money::ZERO,
            optional_commitments: Money::ZERO,
        });
    }
    buckets
}

/// A bill whose pay-by date has already passed. The money is still owed, so the
/// original payment window can no longer decide anything.
pub fn is_past_due(bill: &BillForSchedule, planning_date: NaiveDate) -> bool {
    bill.latest_payment_date < planning_date
}

fn eligible_bucket_indexes(
    bill: &BillForSchedule,
    planning_date: NaiveDate,
    buckets: &[FundingBucket],
) -> Vec<usize> {
    // A past-due bill's window sits entirely behind the planning date, so matching
    // against it would exclude every bucket and leave the bill permanently
    // unfundable. Its effective window instead becomes "as soon as possible":
    // current cash today, or any paycheck from the planning date forward.
    let past_due = is_past_due(bill, planning_date);
    let mut indexes = Vec::new();
    for (index, bucket) in buckets.iter().enumerate() {
        let eligible = match bucket.source_type {
            FundingSourceType::CurrentCash => {
                past_due
                    || (planning_date >= bill.earliest_payment_date
                        && planning_date <= bill.latest_payment_date)
            }
            FundingSourceType::Paycheck => {
                if past_due {
                    bucket.date >= planning_date
                } else {
                    bucket.date >= bill.earliest_payment_date
                        && bucket.date <= bill.latest_payment_date
                }
            }
        };
        if eligible {
            indexes.push(index);
        }
    }
    indexes
}

/// Buckets in the order this bill should prefer to consume them.
///
/// A bill inside its window holds out for the latest eligible source, which keeps
/// cash available as long as possible. A past-due bill inverts that: it is already
/// late, so it takes the earliest money it can reach.
fn funding_preference(
    bill: &BillForSchedule,
    planning_date: NaiveDate,
    eligible: &[usize],
) -> Vec<usize> {
    if is_past_due(bill, planning_date) {
        eligible.to_vec()
    } else {
        eligible.iter().rev().copied().collect()
    }
}

fn payment_date_for(
    bill: &BillForSchedule,
    bucket: &FundingBucket,
    planning_date: NaiveDate,
) -> NaiveDate {
    // A late bill is paid as soon as it is funded. Echoing its original due date
    // back would recommend a payment date that is already in the past.
    if is_past_due(bill, planning_date) {
        return bucket.date.max(planning_date);
    }
    match bill.payment_type {
        PaymentType::Autopay => bill.due_date,
        PaymentType::Manual => bucket.date,
    }
}

fn allocation_for(
    bill: &BillForSchedule,
    bucket: &FundingBucket,
    amount: Money,
    reason_code: ScheduleReasonCode,
    planning_date: NaiveDate,
) -> BillAllocationResult {
    BillAllocationResult {
        bill_id: bill.id.clone(),
        paycheck_id: match bucket.source_type {
            FundingSourceType::CurrentCash => None,
            FundingSourceType::Paycheck => Some(bucket.id.clone()),
        },
        funding_source_type: bucket.source_type,
        amount,
        payment_date: payment_date_for(bill, bucket, planning_date),
        reason_code,
    }
}

fn bucket_by_paycheck_id(buckets: &[FundingBucket], paycheck_id: &str) -> Option<usize> {
    buckets.iter().position(|bucket| {
        bucket.source_type == FundingSourceType::Paycheck && bucket.id == paycheck_id
    })
}

fn push_shortage_warning(
    warnings: &mut Vec<ScheduleWarning>,
    bill: &BillForSchedule,
    bucket: &FundingBucket,
) {
    if bucket.remaining().is_negative() {
        warnings.push(ScheduleWarning {
            code: WarningCode::FundingShortage,
            entity_id: Some(bill.id.clone()),
            date: Some(bucket.date),
            amount: Some(bucket.remaining().abs()),
            message: format!(
                "Bill {} causes the {} funding bucket to exceed available headroom by {} cents.",
                bill.id,
                bucket.id,
                bucket.remaining().abs().value()
            ),
        });
    }
}

fn allocate_bills(
    input: &ScheduleInput,
    buckets: &mut [FundingBucket],
    warnings: &mut Vec<ScheduleWarning>,
) -> (Vec<BillAllocationResult>, Vec<String>) {
    let mut bills = input.bills.clone();
    bills.sort_by_key(|b| (b.latest_payment_date, Reverse(b.priority), b.id.clone()));

    let mut allocations = Vec::new();
    let mut unresolved = Vec::new();

    for bill in bills {
        if bill.amount <= Money::ZERO {
            continue;
        }

        if is_past_due(&bill, input.planning_date) {
            warnings.push(ScheduleWarning {
                code: WarningCode::BillPastDue,
                entity_id: Some(bill.id.clone()),
                date: Some(bill.latest_payment_date),
                amount: Some(bill.amount),
                message: format!(
                    "Bill {} is past its pay-by date and is scheduled from the soonest available funds.",
                    bill.id
                ),
            });
        }

        let eligible = eligible_bucket_indexes(&bill, input.planning_date, buckets);
        if eligible.is_empty() {
            unresolved.push(bill.id.clone());
            warnings.push(ScheduleWarning {
                code: WarningCode::NoEligibleFundingSource,
                entity_id: Some(bill.id.clone()),
                date: Some(bill.latest_payment_date),
                amount: Some(bill.amount),
                message: format!("Bill {} has no eligible funding source in its payment window.", bill.id),
            });
            continue;
        }

        if let Some(locked_id) = bill.locked_paycheck_id.as_deref() {
            let Some(bucket_index) = bucket_by_paycheck_id(buckets, locked_id) else {
                unresolved.push(bill.id.clone());
                warnings.push(ScheduleWarning {
                    code: WarningCode::InvalidLockedPaycheck,
                    entity_id: Some(bill.id.clone()),
                    date: Some(bill.latest_payment_date),
                    amount: Some(bill.amount),
                    message: format!("Bill {} is locked to a paycheck that does not exist.", bill.id),
                });
                continue;
            };
            if !eligible.contains(&bucket_index) {
                unresolved.push(bill.id.clone());
                warnings.push(ScheduleWarning {
                    code: WarningCode::InvalidLockedPaycheck,
                    entity_id: Some(bill.id.clone()),
                    date: Some(bill.latest_payment_date),
                    amount: Some(bill.amount),
                    message: format!("Bill {} is locked outside its valid payment window.", bill.id),
                });
                continue;
            }

            buckets[bucket_index].consume_bill(bill.amount);
            let allocation = allocation_for(
                &bill,
                &buckets[bucket_index],
                bill.amount,
                ScheduleReasonCode::UserLock,
                input.planning_date,
            );
            allocations.push(allocation);
            push_shortage_warning(warnings, &bill, &buckets[bucket_index]);
            continue;
        }

        if let Some(existing_id) = bill.existing_paycheck_id.as_deref() {
            if let Some(bucket_index) = bucket_by_paycheck_id(buckets, existing_id) {
                if eligible.contains(&bucket_index) && buckets[bucket_index].remaining() >= bill.amount {
                    buckets[bucket_index].consume_bill(bill.amount);
                    allocations.push(allocation_for(
                        &bill,
                        &buckets[bucket_index],
                        bill.amount,
                        ScheduleReasonCode::StableExistingAssignment,
                        input.planning_date,
                    ));
                    continue;
                }
            }
        }

        let preference = funding_preference(&bill, input.planning_date, &eligible);
        let preferred_index = *preference.first().expect("eligible is not empty");
        if buckets[preferred_index].remaining() >= bill.amount {
            buckets[preferred_index].consume_bill(bill.amount);
            let reason = match buckets[preferred_index].source_type {
                FundingSourceType::CurrentCash => ScheduleReasonCode::CurrentCash,
                FundingSourceType::Paycheck => match bill.payment_type {
                    PaymentType::Autopay => ScheduleReasonCode::AutopayLatestEligiblePaycheck,
                    PaymentType::Manual => ScheduleReasonCode::LatestEligiblePaycheck,
                },
            };
            allocations.push(allocation_for(
                &bill,
                &buckets[preferred_index],
                bill.amount,
                reason,
                input.planning_date,
            ));
            continue;
        }

        if let Some(fallback_index) = preference
            .iter()
            .copied()
            .skip(1)
            .find(|idx| buckets[*idx].remaining() >= bill.amount)
        {
            buckets[fallback_index].consume_bill(bill.amount);
            allocations.push(allocation_for(
                &bill,
                &buckets[fallback_index],
                bill.amount,
                ScheduleReasonCode::MovedEarlierToProtectBuffer,
                input.planning_date,
            ));
            continue;
        }

        // Funding and payment are deliberately different concepts. Even when a
        // bill itself may only be paid once, the household can reserve money
        // toward it from multiple earlier paychecks. `can_split` controls whether
        // the actual bill payment may be split; it does not prevent multi-paycheck
        // funding/reservation.
        let mut remaining = bill.amount;
        let mut bill_allocations = Vec::new();
        for bucket_index in preference.iter().copied() {
            let available = buckets[bucket_index].remaining().max(Money::ZERO);
            if available.is_zero() {
                continue;
            }
            let part = remaining.min(available);
            buckets[bucket_index].consume_bill(part);
            let reason = if bill.can_split {
                ScheduleReasonCode::SplitAcrossPaychecks
            } else {
                ScheduleReasonCode::ReservedAcrossPaychecks
            };
            let mut allocation =
                allocation_for(&bill, &buckets[bucket_index], part, reason, input.planning_date);
            // A non-splittable bill is still one payment. Earlier paychecks merely
            // reserve funds; the recommended payment date remains the latest safe
            // date in the bill's window. A past-due bill has no such date left, so
            // `allocation_for` already pinned it to the soonest funded date.
            if !bill.can_split && !is_past_due(&bill, input.planning_date) {
                allocation.payment_date = bill.latest_payment_date;
            }
            bill_allocations.push(allocation);
            remaining -= part;
            if remaining.is_zero() {
                break;
            }
        }
        allocations.extend(bill_allocations);
        if remaining.is_positive() {
            unresolved.push(bill.id.clone());
            warnings.push(ScheduleWarning {
                code: WarningCode::PartialFunding,
                entity_id: Some(bill.id.clone()),
                date: Some(bill.latest_payment_date),
                amount: Some(remaining),
                message: format!(
                    "Bill {} still needs {} cents reserved before its payment deadline.",
                    bill.id,
                    remaining.value()
                ),
            });
        }
    }

    (allocations, unresolved)
}

fn commitment_target_bucket(
    commitment: &OptionalCommitmentForSchedule,
    buckets: &[FundingBucket],
) -> Option<usize> {
    if let Some(target_id) = commitment.target_paycheck_id.as_deref() {
        return bucket_by_paycheck_id(buckets, target_id);
    }

    buckets
        .iter()
        .enumerate()
        .filter(|(_, bucket)| bucket.date <= commitment.scheduled_date)
        .max_by_key(|(_, bucket)| bucket.date)
        .map(|(index, _)| index)
}

fn allocate_optional_commitments(
    input: &ScheduleInput,
    buckets: &mut [FundingBucket],
    warnings: &mut Vec<ScheduleWarning>,
) -> Vec<CommitmentAdjustment> {
    let mut prepared = input
        .optional_commitments
        .iter()
        .cloned()
        .map(|commitment| {
            let target = commitment_target_bucket(&commitment, buckets);
            (commitment, target)
        })
        .collect::<Vec<_>>();

    // Within the same date/bucket, protect the higher-ranked commitment first.
    // Extra debt is therefore reduced before optional savings, exactly as the
    // household rules require.
    prepared.sort_by_key(|(commitment, target)| {
        let date = target
            .and_then(|index| buckets.get(index).map(|bucket| bucket.date))
            .unwrap_or(commitment.scheduled_date);
        (date, Reverse(commitment.kind.preservation_rank()), commitment.id.clone())
    });

    let mut adjustments = Vec::new();
    for (commitment, target) in prepared {
        let requested = commitment.amount.max(Money::ZERO);
        let minimum = commitment.minimum_amount.max(Money::ZERO).min(requested);
        let Some(bucket_index) = target else {
            adjustments.push(CommitmentAdjustment {
                commitment_id: commitment.id.clone(),
                kind: commitment.kind,
                requested_amount: requested,
                effective_amount: Money::ZERO,
                reduced_by: requested,
                funding_bucket_id: None,
                scheduled_date: commitment.scheduled_date,
            });
            if requested.is_positive() {
                warnings.push(ScheduleWarning {
                    code: WarningCode::OptionalCommitmentReduced,
                    entity_id: Some(commitment.id),
                    date: Some(commitment.scheduled_date),
                    amount: Some(requested),
                    message: "Optional commitment has no eligible funding bucket and was reduced to zero.".into(),
                });
            }
            continue;
        };

        let available = buckets[bucket_index].remaining().max(Money::ZERO);
        let effective = if available < minimum {
            Money::ZERO
        } else {
            requested.min(available)
        };
        let reduced_by = requested - effective;
        buckets[bucket_index].consume_optional(effective);

        if reduced_by.is_positive() {
            warnings.push(ScheduleWarning {
                code: WarningCode::OptionalCommitmentReduced,
                entity_id: Some(commitment.id.clone()),
                date: Some(buckets[bucket_index].date),
                amount: Some(reduced_by),
                message: format!(
                    "Optional commitment {} was reduced by {} cents to protect required bills and the cash buffer.",
                    commitment.id,
                    reduced_by.value()
                ),
            });
        }

        adjustments.push(CommitmentAdjustment {
            commitment_id: commitment.id,
            kind: commitment.kind,
            requested_amount: requested,
            effective_amount: effective,
            reduced_by,
            funding_bucket_id: Some(buckets[bucket_index].id.clone()),
            scheduled_date: buckets[bucket_index].date,
        });
    }
    adjustments
}

fn bucket_summaries(input: &ScheduleInput, buckets: &[FundingBucket]) -> Vec<FundingBucketSummary> {
    buckets
        .iter()
        .map(|bucket| {
            let remaining = bucket.remaining();
            let status = if remaining.is_negative() {
                BucketStatus::Shortage
            } else if remaining < input.tight_headroom.max(Money::ZERO) {
                BucketStatus::Tight
            } else {
                BucketStatus::Healthy
            };
            FundingBucketSummary {
                bucket_id: bucket.id.clone(),
                funding_source_type: bucket.source_type,
                date: bucket.date,
                gross_amount: bucket.gross_amount,
                buffer_replenishment: bucket.buffer_replenishment,
                allocatable_capacity: bucket.capacity,
                bill_allocations: bucket.bill_allocations,
                optional_commitments: bucket.optional_commitments,
                remaining_headroom: remaining,
                status,
            }
        })
        .collect()
}

#[derive(Debug, Clone)]
struct RawProjectionEvent {
    date: NaiveDate,
    event_type: ProjectionEventType,
    entity_id: Option<String>,
    amount: Money,
    sort_order: u8,
}

fn build_projection(
    input: &ScheduleInput,
    allocations: &[BillAllocationResult],
    commitments: &[CommitmentAdjustment],
    warnings: &mut Vec<ScheduleWarning>,
) -> Vec<ProjectionEvent> {
    let mut raw = Vec::new();

    for paycheck in &input.paychecks {
        raw.push(RawProjectionEvent {
            date: paycheck.pay_date,
            event_type: ProjectionEventType::Paycheck,
            entity_id: Some(paycheck.id.clone()),
            amount: paycheck.amount,
            sort_order: 10,
        });
    }

    let allocations_by_bill = allocations.iter().fold(
        HashMap::<String, Vec<&BillAllocationResult>>::new(),
        |mut map, allocation| {
            map.entry(allocation.bill_id.clone()).or_default().push(allocation);
            map
        },
    );

    for bill in &input.bills {
        match bill.payment_type {
            PaymentType::Autopay => {
                raw.push(RawProjectionEvent {
                    date: bill.due_date,
                    event_type: ProjectionEventType::BillPayment,
                    entity_id: Some(bill.id.clone()),
                    amount: -bill.amount,
                    sort_order: 20,
                });
            }
            PaymentType::Manual => {
                let bill_allocations = allocations_by_bill.get(&bill.id).cloned().unwrap_or_default();
                let funded = bill_allocations
                    .iter()
                    .fold(Money::ZERO, |total, allocation| total + allocation.amount);
                for allocation in bill_allocations {
                    raw.push(RawProjectionEvent {
                        date: allocation.payment_date,
                        event_type: ProjectionEventType::BillPayment,
                        entity_id: Some(bill.id.clone()),
                        amount: -allocation.amount,
                        sort_order: 20,
                    });
                }
                let remaining = (bill.amount - funded).max(Money::ZERO);
                if remaining.is_positive() {
                    raw.push(RawProjectionEvent {
                        date: bill.latest_payment_date,
                        event_type: ProjectionEventType::BillPayment,
                        entity_id: Some(bill.id.clone()),
                        amount: -remaining,
                        sort_order: 20,
                    });
                }
            }
        }
    }

    for commitment in commitments {
        if commitment.effective_amount.is_positive() {
            raw.push(RawProjectionEvent {
                date: commitment.scheduled_date,
                event_type: ProjectionEventType::OptionalCommitment,
                entity_id: Some(commitment.commitment_id.clone()),
                amount: -commitment.effective_amount,
                sort_order: 30,
            });
        }
    }

    raw.sort_by_key(|event| (event.date, event.sort_order, event.entity_id.clone()));

    let mut balance = input.starting_cash;
    let mut projection = vec![ProjectionEvent {
        date: input.planning_date,
        event_type: ProjectionEventType::StartingBalance,
        entity_id: None,
        amount: Money::ZERO,
        balance_after: balance,
        safe_to_spend_after: safe_to_spend(balance, input.protected_buffer),
    }];

    let mut end_of_day_balances = BTreeMap::<NaiveDate, Money>::new();

    // The starting balance is an actual current condition, so surface it on the
    // planning date. Future dates are evaluated only after every event on that
    // date has been applied. This avoids false warnings between two paychecks
    // (or a paycheck and bill) that happen on the same day.
    end_of_day_balances.insert(input.planning_date, balance);

    for event in raw {
        balance += event.amount;
        let safe = safe_to_spend(balance, input.protected_buffer);
        projection.push(ProjectionEvent {
            date: event.date,
            event_type: event.event_type,
            entity_id: event.entity_id,
            amount: event.amount,
            balance_after: balance,
            safe_to_spend_after: safe,
        });
        end_of_day_balances.insert(event.date, balance);
    }

    for (date, end_balance) in end_of_day_balances {
        if end_balance < input.protected_buffer {
            let shortfall = input.protected_buffer - end_balance;
            warnings.push(ScheduleWarning {
                code: WarningCode::ProjectedBelowBuffer,
                entity_id: None,
                date: Some(date),
                amount: Some(shortfall),
                message: format!(
                    "Projected end-of-day balance is {} below the protected buffer on {}.",
                    format_money(shortfall),
                    date
                ),
            });
        }
        if end_balance.is_negative() {
            let shortfall = end_balance.abs();
            warnings.push(ScheduleWarning {
                code: WarningCode::ProjectedNegativeBalance,
                entity_id: None,
                date: Some(date),
                amount: Some(shortfall),
                message: format!(
                    "Projected end-of-day balance is {} below $0.00 on {}.",
                    format_money(shortfall),
                    date
                ),
            });
        }
    }

    projection
}

pub fn build_plan(input: &ScheduleInput) -> ScheduleResult {
    let mut buckets = build_funding_buckets(input);
    let mut warnings = Vec::new();
    let (allocations, unresolved_bill_ids) = allocate_bills(input, &mut buckets, &mut warnings);
    let commitment_adjustments =
        allocate_optional_commitments(input, &mut buckets, &mut warnings);
    let bucket_summaries = bucket_summaries(input, &buckets);
    let projection = build_projection(
        input,
        &allocations,
        &commitment_adjustments,
        &mut warnings,
    );

    ScheduleResult {
        allocations,
        unresolved_bill_ids,
        bucket_summaries,
        commitment_adjustments,
        projection,
        warnings,
    }
}

pub fn simulate_paycheck_amount_change(
    input: &ScheduleInput,
    paycheck_id: &str,
    new_amount: Money,
) -> Option<PaycheckChangeSimulation> {
    let old_amount = input
        .paychecks
        .iter()
        .find(|paycheck| paycheck.id == paycheck_id)?
        .amount;
    let baseline = build_plan(input);
    let mut changed = input.clone();
    let effective_new_amount = new_amount.max(Money::ZERO);
    {
        let paycheck = changed
            .paychecks
            .iter_mut()
            .find(|paycheck| paycheck.id == paycheck_id)?;
        paycheck.amount = effective_new_amount;
    }
    let scenario = build_plan(&changed);
    Some(PaycheckChangeSimulation {
        paycheck_id: paycheck_id.into(),
        old_amount,
        new_amount: effective_new_amount,
        baseline,
        scenario,
    })
}

pub fn simulate_manual_move(
    input: &ScheduleInput,
    bill_id: &str,
    target_paycheck_id: &str,
) -> Option<ManualMoveSimulation> {
    let baseline = build_plan(input);
    let source_bucket_id = baseline
        .allocations
        .iter()
        .find(|allocation| allocation.bill_id == bill_id)
        .map(|allocation| {
            allocation
                .paycheck_id
                .clone()
                .unwrap_or_else(|| CURRENT_CASH_BUCKET_ID.into())
        });

    let target_exists = input
        .paychecks
        .iter()
        .any(|paycheck| paycheck.id == target_paycheck_id);
    let bill = input.bills.iter().find(|bill| bill.id == bill_id)?;
    let target_date = input
        .paychecks
        .iter()
        .find(|paycheck| paycheck.id == target_paycheck_id)
        .map(|paycheck| paycheck.pay_date);
    let valid = target_exists
        && target_date
            .map(|date| date >= bill.earliest_payment_date && date <= bill.latest_payment_date)
            .unwrap_or(false);

    let before_map = baseline
        .bucket_summaries
        .iter()
        .map(|summary| (summary.bucket_id.clone(), summary.remaining_headroom))
        .collect::<HashMap<_, _>>();

    if !valid {
        return Some(ManualMoveSimulation {
            bill_id: bill_id.into(),
            source_bucket_id: source_bucket_id.clone(),
            target_paycheck_id: target_paycheck_id.into(),
            valid: false,
            source_headroom_before: source_bucket_id
                .as_ref()
                .and_then(|id| before_map.get(id).copied()),
            source_headroom_after: None,
            target_headroom_before: before_map.get(target_paycheck_id).copied(),
            target_headroom_after: None,
            scenario_warnings: Vec::new(),
        });
    }

    let mut scenario_input = input.clone();
    let changed_bill = scenario_input
        .bills
        .iter_mut()
        .find(|candidate| candidate.id == bill_id)?;
    changed_bill.locked_paycheck_id = Some(target_paycheck_id.into());
    changed_bill.existing_paycheck_id = None;
    let scenario = build_plan(&scenario_input);
    let after_map = scenario
        .bucket_summaries
        .iter()
        .map(|summary| (summary.bucket_id.clone(), summary.remaining_headroom))
        .collect::<HashMap<_, _>>();

    Some(ManualMoveSimulation {
        bill_id: bill_id.into(),
        source_bucket_id: source_bucket_id.clone(),
        target_paycheck_id: target_paycheck_id.into(),
        valid: true,
        source_headroom_before: source_bucket_id
            .as_ref()
            .and_then(|id| before_map.get(id).copied()),
        source_headroom_after: source_bucket_id
            .as_ref()
            .and_then(|id| after_map.get(id).copied()),
        target_headroom_before: before_map.get(target_paycheck_id).copied(),
        target_headroom_after: after_map.get(target_paycheck_id).copied(),
        scenario_warnings: scenario.warnings,
    })
}

pub fn prior_business_day(date: NaiveDate) -> NaiveDate {
    recurrence::prior_business_day(date)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::models::Priority;

    fn d(value: &str) -> NaiveDate {
        NaiveDate::parse_from_str(value, "%Y-%m-%d").unwrap()
    }

    fn p(id: &str, date: &str, amount: i64) -> PaycheckForSchedule {
        PaycheckForSchedule {
            id: id.into(),
            pay_date: d(date),
            amount: Money::cents(amount),
        }
    }

    fn bill(
        id: &str,
        due: &str,
        amount: i64,
        payment_type: PaymentType,
    ) -> BillForSchedule {
        let due_date = d(due);
        BillForSchedule {
            id: id.into(),
            due_date,
            earliest_payment_date: d("2026-08-01"),
            latest_payment_date: due_date,
            amount: Money::cents(amount),
            payment_type,
            priority: Priority::Normal,
            locked_paycheck_id: None,
            existing_paycheck_id: None,
            can_split: false,
        }
    }

    fn input(
        starting_cash: i64,
        protected_buffer: i64,
        paychecks: Vec<PaycheckForSchedule>,
        bills: Vec<BillForSchedule>,
    ) -> ScheduleInput {
        ScheduleInput {
            planning_date: d("2026-08-01"),
            starting_cash: Money::cents(starting_cash),
            protected_buffer: Money::cents(protected_buffer),
            tight_headroom: Money::cents(10_000),
            paychecks,
            bills,
            optional_commitments: Vec::new(),
        }
    }

    fn bucket<'a>(result: &'a ScheduleResult, id: &str) -> &'a FundingBucketSummary {
        result
            .bucket_summaries
            .iter()
            .find(|summary| summary.bucket_id == id)
            .unwrap()
    }

    #[test]
    fn protected_buffer_is_floor_not_expense() {
        assert_eq!(
            safe_to_spend(Money::cents(100_000), Money::cents(50_000)),
            Money::cents(50_000)
        );
        let result = build_plan(&input(
            100_000,
            50_000,
            vec![p("p1", "2026-08-14", 100_000), p("p2", "2026-08-28", 100_000)],
            vec![],
        ));
        assert_eq!(bucket(&result, CURRENT_CASH_BUCKET_ID).allocatable_capacity, Money::cents(50_000));
        assert_eq!(bucket(&result, "p1").allocatable_capacity, Money::cents(100_000));
        assert_eq!(bucket(&result, "p2").allocatable_capacity, Money::cents(100_000));
    }

    #[test]
    fn paycheck_replenishes_buffer_deficit_only_once() {
        let result = build_plan(&input(
            20_000,
            50_000,
            vec![p("p1", "2026-08-14", 100_000), p("p2", "2026-08-28", 100_000)],
            vec![],
        ));
        assert_eq!(bucket(&result, "p1").buffer_replenishment, Money::cents(30_000));
        assert_eq!(bucket(&result, "p1").allocatable_capacity, Money::cents(70_000));
        assert_eq!(bucket(&result, "p2").buffer_replenishment, Money::ZERO);
        assert_eq!(bucket(&result, "p2").allocatable_capacity, Money::cents(100_000));
    }

    #[test]
    fn variable_bill_uses_last_six_average() {
        let values = [17_381, 16_244, 17_622, 16_890, 15_933, 16_177].map(Money::cents);
        assert_eq!(estimate_variable_bill(&values, Money::ZERO), Money::cents(16_708));
    }

    #[test]
    fn variable_bill_estimate_rounds_half_up_rather_than_truncating() {
        // 100_247 / 6 = 16_707.83. Truncation gives 16_707, which is what the
        // production path used to return while this helper returned 16_708.
        let values = [17_381, 16_244, 17_622, 16_890, 15_933, 16_177].map(Money::cents);
        let truncated = values.iter().map(|m| m.value()).sum::<i64>() / values.len() as i64;
        assert_eq!(truncated, 16_707);
        assert_eq!(estimate_variable_bill(&values, Money::ZERO), Money::cents(16_708));
    }

    #[test]
    fn variable_bill_estimate_averages_exactly_what_it_is_given() {
        // Windowing belongs to the caller, so more than six values are all averaged.
        let values = [10_000, 20_000, 30_000, 40_000, 50_000, 60_000, 70_000].map(Money::cents);
        assert_eq!(estimate_variable_bill(&values, Money::ZERO), Money::cents(40_000));
    }

    #[test]
    fn variable_bill_estimate_falls_back_when_there_is_no_history() {
        assert_eq!(
            estimate_variable_bill(&[], Money::cents(12_345)),
            Money::cents(12_345)
        );
    }

    #[test]
    fn latest_eligible_paycheck_is_selected() {
        let paychecks = vec![
            p("p1", "2026-08-01", 200_000),
            p("p2", "2026-08-14", 200_000),
            p("p3", "2026-08-28", 200_000),
        ];
        let b = bill("b1", "2026-08-25", 20_000, PaymentType::Manual);
        assert_eq!(latest_eligible_paycheck(&b, &paychecks).unwrap().id, "p2");
    }

    #[test]
    fn basic_latest_paycheck_assignment() {
        let result = build_plan(&input(
            50_000,
            50_000,
            vec![p("p1", "2026-08-14", 200_000), p("p2", "2026-08-21", 150_000)],
            vec![bill("electric", "2026-08-18", 20_000, PaymentType::Manual)],
        ));
        assert_eq!(result.allocations[0].paycheck_id.as_deref(), Some("p1"));
        assert_eq!(result.allocations[0].payment_date, d("2026-08-14"));
    }

    #[test]
    fn moves_manual_bill_earlier_to_protect_later_paycheck_headroom() {
        let mut fixed = bill("fixed", "2026-08-22", 45_000, PaymentType::Manual);
        fixed.earliest_payment_date = d("2026-08-14");
        let mut electric = bill("electric", "2026-08-23", 18_422, PaymentType::Manual);
        electric.earliest_payment_date = d("2026-08-14");
        let result = build_plan(&input(
            50_000,
            50_000,
            vec![p("p1", "2026-08-14", 100_000), p("p2", "2026-08-21", 50_000)],
            vec![fixed, electric],
        ));
        let electric_allocation = result
            .allocations
            .iter()
            .find(|allocation| allocation.bill_id == "electric")
            .unwrap();
        assert_eq!(electric_allocation.paycheck_id.as_deref(), Some("p1"));
        assert_eq!(electric_allocation.reason_code, ScheduleReasonCode::MovedEarlierToProtectBuffer);
    }

    #[test]
    fn autopay_funding_does_not_change_draft_date() {
        let paychecks = vec![p("p1", "2026-08-14", 200_000), p("p2", "2026-08-21", 150_000)];
        let mut b = bill("netflix", "2026-08-23", 2_400, PaymentType::Autopay);
        b.earliest_payment_date = d("2026-08-01");
        let result = build_plan(&input(50_000, 50_000, paychecks, vec![b]));
        assert_eq!(result.allocations[0].paycheck_id.as_deref(), Some("p2"));
        assert_eq!(result.allocations[0].payment_date, d("2026-08-23"));
        assert_eq!(
            result.allocations[0].reason_code,
            ScheduleReasonCode::AutopayLatestEligiblePaycheck
        );
    }

    #[test]
    fn existing_valid_assignment_is_stable() {
        let mut b = bill("internet", "2026-08-23", 9_000, PaymentType::Manual);
        b.existing_paycheck_id = Some("p1".into());
        let result = build_plan(&input(
            50_000,
            50_000,
            vec![p("p1", "2026-08-14", 100_000), p("p2", "2026-08-21", 100_000)],
            vec![b],
        ));
        assert_eq!(result.allocations[0].paycheck_id.as_deref(), Some("p1"));
        assert_eq!(result.allocations[0].reason_code, ScheduleReasonCode::StableExistingAssignment);
    }

    #[test]
    fn locked_bill_is_preserved_even_when_bucket_is_short() {
        let mut b = bill("mortgage", "2026-08-25", 50_000, PaymentType::Manual);
        b.locked_paycheck_id = Some("p1".into());
        let result = build_plan(&input(
            50_000,
            50_000,
            vec![p("p1", "2026-08-14", 10_000), p("p2", "2026-08-21", 100_000)],
            vec![b],
        ));
        assert_eq!(result.allocations[0].paycheck_id.as_deref(), Some("p1"));
        assert_eq!(bucket(&result, "p1").status, BucketStatus::Shortage);
        assert!(result.warnings.iter().any(|warning| warning.code == WarningCode::FundingShortage));
    }

    #[test]
    fn invalid_lock_warns_instead_of_silently_moving() {
        let mut b = bill("mortgage", "2026-08-18", 50_000, PaymentType::Manual);
        b.locked_paycheck_id = Some("p2".into());
        let result = build_plan(&input(
            50_000,
            50_000,
            vec![p("p1", "2026-08-14", 100_000), p("p2", "2026-08-21", 100_000)],
            vec![b],
        ));
        assert!(result.allocations.is_empty());
        assert_eq!(result.unresolved_bill_ids, vec!["mortgage"]);
        assert!(result.warnings.iter().any(|warning| warning.code == WarningCode::InvalidLockedPaycheck));
    }

    #[test]
    fn non_split_bill_can_be_reserved_across_multiple_paychecks_but_paid_once() {
        let mut b = bill("mortgage", "2026-09-20", 220_000, PaymentType::Manual);
        b.earliest_payment_date = d("2026-08-20");
        b.latest_payment_date = d("2026-09-18");
        b.can_split = false;
        let result = build_plan(&input(
            50_000,
            50_000,
            vec![
                p("p1", "2026-09-11", 187_600),
                p("p2", "2026-09-18", 67_000),
            ],
            vec![b],
        ));
        let parts = result.allocations.iter().filter(|a| a.bill_id == "mortgage").collect::<Vec<_>>();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts.iter().fold(Money::ZERO, |sum, p| sum + p.amount), Money::cents(220_000));
        assert!(parts.iter().all(|p| p.payment_date == d("2026-09-18")));
        assert!(parts.iter().all(|p| p.reason_code == ScheduleReasonCode::ReservedAcrossPaychecks));
        assert!(result.unresolved_bill_ids.is_empty());
        assert!(!result.warnings.iter().any(|w| matches!(w.code, WarningCode::FundingShortage | WarningCode::PartialFunding)));
    }

    #[test]
    fn split_bill_can_use_multiple_paychecks() {
        let mut b = bill("card", "2026-08-28", 60_000, PaymentType::Manual);
        b.earliest_payment_date = d("2026-08-14");
        b.can_split = true;
        let result = build_plan(&input(
            50_000,
            50_000,
            vec![p("p1", "2026-08-14", 30_000), p("p2", "2026-08-21", 30_000)],
            vec![b],
        ));
        let parts = result
            .allocations
            .iter()
            .filter(|allocation| allocation.bill_id == "card")
            .collect::<Vec<_>>();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts.iter().fold(Money::ZERO, |sum, part| sum + part.amount), Money::cents(60_000));
        assert!(result.unresolved_bill_ids.is_empty());
    }

    #[test]
    fn optional_extra_debt_is_reduced_before_savings() {
        let mut schedule = input(
            50_000,
            50_000,
            vec![p("p1", "2026-08-14", 20_000)],
            vec![],
        );
        schedule.optional_commitments = vec![
            OptionalCommitmentForSchedule {
                id: "extra-debt".into(),
                scheduled_date: d("2026-08-14"),
                target_paycheck_id: Some("p1".into()),
                amount: Money::cents(10_000),
                minimum_amount: Money::ZERO,
                kind: OptionalCommitmentKind::ExtraDebt,
            },
            OptionalCommitmentForSchedule {
                id: "savings".into(),
                scheduled_date: d("2026-08-14"),
                target_paycheck_id: Some("p1".into()),
                amount: Money::cents(20_000),
                minimum_amount: Money::ZERO,
                kind: OptionalCommitmentKind::OptionalSavings,
            },
        ];
        let result = build_plan(&schedule);
        let extra = result
            .commitment_adjustments
            .iter()
            .find(|item| item.commitment_id == "extra-debt")
            .unwrap();
        let savings = result
            .commitment_adjustments
            .iter()
            .find(|item| item.commitment_id == "savings")
            .unwrap();
        assert_eq!(savings.effective_amount, Money::cents(20_000));
        assert_eq!(extra.effective_amount, Money::ZERO);
    }

    #[test]
    fn lower_paycheck_recalculates_and_reduces_optional_commitments() {
        let mut schedule = input(
            50_000,
            50_000,
            vec![p("p1", "2026-08-14", 200_000)],
            vec![bill("mortgage", "2026-08-15", 110_000, PaymentType::Manual)],
        );
        schedule.optional_commitments = vec![OptionalCommitmentForSchedule {
            id: "extra-debt".into(),
            scheduled_date: d("2026-08-14"),
            target_paycheck_id: Some("p1".into()),
            amount: Money::cents(50_000),
            minimum_amount: Money::ZERO,
            kind: OptionalCommitmentKind::ExtraDebt,
        }];
        let simulation = simulate_paycheck_amount_change(&schedule, "p1", Money::cents(140_000)).unwrap();
        let baseline_extra = simulation
            .baseline
            .commitment_adjustments
            .iter()
            .find(|item| item.commitment_id == "extra-debt")
            .unwrap();
        let scenario_extra = simulation
            .scenario
            .commitment_adjustments
            .iter()
            .find(|item| item.commitment_id == "extra-debt")
            .unwrap();
        assert_eq!(baseline_extra.effective_amount, Money::cents(50_000));
        assert_eq!(scenario_extra.effective_amount, Money::cents(30_000));
    }

    #[test]
    fn projection_orders_paycheck_before_same_day_manual_bill() {
        let mut b = bill("mortgage", "2026-08-15", 110_000, PaymentType::Manual);
        b.earliest_payment_date = d("2026-08-14");
        let result = build_plan(&input(
            50_000,
            50_000,
            vec![p("p1", "2026-08-14", 200_000)],
            vec![b],
        ));
        let aug14 = result
            .projection
            .iter()
            .filter(|event| event.date == d("2026-08-14"))
            .collect::<Vec<_>>();
        assert_eq!(aug14[0].event_type, ProjectionEventType::Paycheck);
        assert_eq!(aug14[1].event_type, ProjectionEventType::BillPayment);
        assert_eq!(aug14[1].balance_after, Money::cents(140_000));
    }

    #[test]
    fn same_day_paychecks_use_end_of_day_balance_for_warnings() {
        let result = build_plan(&input(
            -295_000,
            30_000,
            vec![
                p("p1", "2026-08-14", 200_000),
                p("p2", "2026-08-14", 200_000),
                p("p3", "2026-08-14", 67_000),
            ],
            vec![],
        ));
        assert!(result.warnings.iter().any(|warning| warning.date == Some(d("2026-08-01"))));
        assert!(!result.warnings.iter().any(|warning| warning.date == Some(d("2026-08-14"))));
        assert!(result.warnings.iter().all(|warning| !warning.message.contains(" cents")));
    }

    #[test]
    fn manual_move_simulation_shows_target_impact() {
        let mut b = bill("electric", "2026-08-23", 18_422, PaymentType::Manual);
        b.earliest_payment_date = d("2026-08-14");
        let schedule = input(
            50_000,
            50_000,
            vec![p("p1", "2026-08-14", 100_000), p("p2", "2026-08-21", 50_000)],
            vec![b],
        );
        let simulation = simulate_manual_move(&schedule, "electric", "p1").unwrap();
        assert!(simulation.valid);
        assert!(simulation.target_headroom_after.unwrap() < simulation.target_headroom_before.unwrap());
    }

    #[test]
    fn weekend_date_moves_to_prior_business_day() {
        assert_eq!(prior_business_day(d("2026-08-16")), d("2026-08-14"));
    }

    fn past_due_bill(id: &str, earliest: &str, latest: &str, amount: i64) -> BillForSchedule {
        let mut b = bill(id, latest, amount, PaymentType::Manual);
        b.earliest_payment_date = d(earliest);
        b.latest_payment_date = d(latest);
        b
    }

    #[test]
    fn past_due_bill_is_funded_instead_of_permanently_unresolvable() {
        // Regression: a bill whose pay-by date had passed matched no funding bucket
        // at all, so it reported NoEligibleFundingSource on every run and no user
        // action could ever fund it.
        let overdue = past_due_bill("late-electric", "2026-07-10", "2026-07-20", 18_422);
        let result = build_plan(&input(
            100_000,
            50_000,
            vec![p("p1", "2026-08-14", 268_012)],
            vec![overdue],
        ));

        assert!(
            result.unresolved_bill_ids.is_empty(),
            "a past-due bill must still be schedulable, got {:?}",
            result.unresolved_bill_ids
        );
        assert!(result
            .warnings
            .iter()
            .any(|w| w.code == WarningCode::BillPastDue));
        assert!(!result
            .warnings
            .iter()
            .any(|w| w.code == WarningCode::NoEligibleFundingSource));

        let allocation = result
            .allocations
            .iter()
            .find(|a| a.bill_id == "late-electric")
            .expect("past-due bill must receive an allocation");
        assert_eq!(allocation.funding_source_type, FundingSourceType::CurrentCash);
        assert_eq!(allocation.amount, Money::cents(18_422));
        assert!(
            allocation.payment_date >= d("2026-08-01"),
            "must not recommend a payment date in the past, got {}",
            allocation.payment_date
        );
    }

    #[test]
    fn past_due_bill_takes_the_soonest_paycheck_when_cash_is_short() {
        // Inside its window a bill holds out for the latest eligible paycheck.
        // A late bill inverts that and grabs the earliest money it can reach.
        let overdue = past_due_bill("late-rent", "2026-07-01", "2026-07-15", 110_000);
        let result = build_plan(&input(
            50_000, // entirely consumed by the protected buffer
            50_000,
            vec![p("p1", "2026-08-14", 268_012), p("p2", "2026-08-28", 268_012)],
            vec![overdue],
        ));

        assert!(result.unresolved_bill_ids.is_empty());
        let allocation = result
            .allocations
            .iter()
            .find(|a| a.bill_id == "late-rent")
            .expect("past-due bill must receive an allocation");
        assert_eq!(
            allocation.paycheck_id.as_deref(),
            Some("p1"),
            "a late bill takes the soonest paycheck, not the latest"
        );
    }

    #[test]
    fn bill_inside_its_window_still_prefers_the_latest_eligible_paycheck() {
        // Guards the fix above from inverting normal scheduling.
        let mut b = bill("electric", "2026-08-30", 18_422, PaymentType::Manual);
        b.earliest_payment_date = d("2026-08-01");
        b.latest_payment_date = d("2026-08-30");
        let result = build_plan(&input(
            100_000,
            50_000,
            vec![p("p1", "2026-08-14", 268_012), p("p2", "2026-08-28", 268_012)],
            vec![b],
        ));

        let allocation = result
            .allocations
            .iter()
            .find(|a| a.bill_id == "electric")
            .expect("bill must receive an allocation");
        assert_eq!(allocation.paycheck_id.as_deref(), Some("p2"));
    }
}
