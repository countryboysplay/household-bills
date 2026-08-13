# Phase 5 Implementation

Version: 0.5.0

Phase 5 is the final functional build before installer/release hardening. Local AI has been removed from the roadmap.

## 1. Explicit bill-payment guidance

The deterministic scheduler now feeds a dedicated payment-guidance view that separates two concepts:

- **Funding / reserving:** which paycheck(s) supply the money for a bill.
- **Payment action:** the date the household should actually pay the bill, or the fixed autopay draft date.

The Dashboard now includes **What to Pay**. The Paycheck Planner includes **Payment Instructions**. Bill Detail shows the recommended payment date and all funding sources. Calendar shows the actual due date and a separate recommended-payment event.

For a non-splittable bill funded across multiple paychecks, the app can reserve portions from several checks while still giving one actual payment date. If the user explicitly enables partial payments for a bill, each scheduler allocation also preserves its recommended provider-payment date, so What to Pay, Payment Instructions, Bill Detail, and Calendar can show the separate partial-payment actions accurately.

## 2. Savings & Debt

- savings goals
- emergency funds
- sinking funds
- target amounts and dates
- planned per-paycheck or monthly contributions
- record actual contributions
- debt balances, APR, minimum payment reference, and optional extra payment
- snowball and avalanche comparison
- actual extra debt-payment tracking
- optional commitments are reduced before required bills or the protected cash buffer

Minimum required debt payments should continue to be entered as Bills so the required-payment scheduler protects them.

## 3. Settings

- household name
- primary bill account name
- protected cash buffer
- 30/60/90/180/365-day planning horizon
- household member display names
- backup retention count
- local data paths

Changing scheduler settings triggers recalculation.

## 4. Backup & Restore

- automatic daily local backup
- configurable retention
- manual backup creation
- pre-Phase-5 migration backup on upgrade
- list recent backups
- restore selected backup on next app launch
- create an additional safety backup before scheduling a restore

## 5. Reports & Exports

- date-range report
- income
- bill payments
- everyday spending
- savings contributions
- extra debt payments
- net cash flow
- spending by category
- monthly cash-flow breakdown
- local CSV export

## 6. Scope decisions

- no bank connection
- no cloud backend
- no AI phase
- no automatic money movement
- no mobile-specific application
- financial calculations remain deterministic Rust logic
