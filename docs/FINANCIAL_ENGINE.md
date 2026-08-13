# Deterministic Financial Engine

> **Phase 5 scope override (current):** AI has been removed from the Household Bills roadmap. Any AI, llama.cpp, model, Ask AI, or AI proposal material later in this historical document is superseded by `AI_SPEC.md` and must not be implemented. The product remains deterministic and local.

## 1. Purpose

The financial engine is the most important part of the application.

It must be deterministic, testable, explainable, and independent of the local LLM.

Given the same financial snapshot and configuration, it should produce the same plan.

## 2. Money

Represent authoritative money values as integer cents.

Never use JavaScript floating-point arithmetic for core totals.

Rust owns authoritative calculations.

## 3. Definitions

### Book balance

The current manually maintained balance for an account inside the app.

It changes through:

- entered transactions
- confirmed paychecks
- confirmed bill payments
- transfers
- reconciliations/adjustments

### Projected balance

Chronological balance produced from the current book balance plus future scheduled inflows/outflows.

### Protected cash buffer

A household-defined minimum balance floor.

Example:

```text
Protected buffer = $500.00
```

The buffer is **not an expense** and must not be subtracted repeatedly from each paycheck.

A plan is buffer-safe when all required projected balances in the relevant planning period are at or above the buffer.

### Reserved money

Money logically set aside for known obligations that have not yet been paid.

Reservations are planning data, not duplicate ledger transactions.

### Safe to spend

The amount that can be spent without causing the projected balance to fall below the protected buffer after accounting for currently reserved required obligations.

For the current active pay period, define:

```text
safe_to_spend =
  max(
    0,
    minimum_projected_balance_before_next_unallocated_income
      - protected_buffer
  )
```

The projection used here must include already planned required bills/autopays and any explicitly reserved optional commitments.

If a shortage exists, safe-to-spend is `0`, and the shortage is shown separately.

For future paycheck cards, show the headroom at the end/lowest point of that paycheck's funding interval, not the paycheck amount minus a fictional repeated buffer expense.

## 4. Scheduling inputs

The scheduler receives a snapshot containing:

- current primary account balance
- protected buffer
- planning horizon
- paycheck occurrences
- bill occurrences
- bill priorities
- payment windows
- autopay dates
- current payments
- manual allocations
- locks
- savings/sinking contributions
- debt minimums
- optional extra debt payments
- planned discretionary allowances
- one-time expenses

## 5. Planning horizon

Default: 90 days.

Supported choices:

- 30
- 60
- 90
- 180
- 365 days

Generate recurring occurrences only far enough ahead to cover the active horizon plus a small safety margin.

## 6. Paycheck amount precedence

For a paycheck occurrence, the effective planning amount is:

1. `actual_amount`, if confirmed
2. `expected_amount`, if manually entered
3. `projected_amount`, generated from income source default

Status should make the source obvious.

## 7. Bill amount precedence

For a bill occurrence, effective amount is:

1. actual amount when fully known
2. occurrence-specific manual estimate/override
3. deterministic variable estimate
4. fixed template amount

### Default variable estimate

Use average of last six fully paid occurrences.

If fewer than six exist, average the available fully paid occurrences.

If none exist, use the template estimate.

Round to cents.

## 8. Payment windows

Each manual bill has:

- earliest permissible payment date
- latest permissible payment date

Defaults:

- earliest: assigned paycheck date or any prior eligible paycheck depending on configuration
- latest: due date adjusted for weekend/holiday rule

Supported bill behaviors:

- Pay Anytime
- Pay Near Due Date
- Custom Window

Autopay's payment/draft date is fixed.

## 9. Weekend and holiday handling

Per bill, allow an explicit behavior.

Default safety behavior:

- if a due date falls on a weekend/recognized configured holiday and no bill-specific rule exists, the latest payment date is the prior business day

Allow one-off user override.

Do not silently invent provider-specific grace behavior.

## 10. Paycheck date handling

Recurring pay schedules generate occurrences.

Allow:

- weekend/holiday rule
- manual per-occurrence date override
- manual amount override

The user may enter a paycheck week to week.

## 11. Bill priorities

Simple priority classes:

1. Essential
2. Normal
3. Flexible

Examples of essential by default:

- housing
- utilities
- insurance
- minimum debt payments

User can override priority.

Priority affects shortage resolution, not ordinary due-date rules.

## 12. Autopay

Autopay occurrences are fixed cash-flow events.

The scheduler:

- reserves for them
- ensures prior paychecks fund them
- may move manual bills around them

It does not change the autopay draft date unless the user edits that occurrence/template.

## 13. Core allocation algorithm

The implementation can optimize internally, but behavior must match these rules.

### Step A: Materialize immutable/manual decisions

Apply:

- already paid amounts
- manual allocations
- occurrence locks
- permanent/recurring assignment rules
- autopay fixed events

Validate them.

If a manual/locked rule makes a bill late, do not silently move it. Return a warning requiring user action.

### Step B: Build eligible funding paychecks

For each unpaid obligation, determine paychecks that:

- occur early enough to fund the bill before latest payment date
- do not violate the bill's earliest-payment/funding rules
- are within the planning horizon

For bills due before any future paycheck, current available cash acts as the funding source.

### Step C: Preferred assignment

For each unassigned required bill, prefer the **latest eligible paycheck** before it must be paid.

This minimizes unnecessarily early payment/reservation.

### Step D: Evaluate projected balance

Create a chronological projection using:

- current book balance
- actual/expected/projected paychecks
- scheduled payment dates
- autopays
- confirmed payments
- other planned transactions

Check the minimum projected balance against the protected buffer.

### Step E: Rebalance earlier if needed

If a future interval falls below the buffer:

1. Identify movable manual bills funded by the stressed paycheck/interval.
2. Consider moving eligible bills to earlier paychecks with spare headroom.
3. Choose moves that improve the shortage without creating an earlier shortage.
4. Respect locks and payment windows.
5. Record reason codes for every automatic move.

Do not move a bill later than its latest permissible payment date.

### Step F: Reduce optional commitments

If required obligations still create a shortage, reduce/pause optional items in this order, subject to user configuration:

1. extra debt payments
2. optional savings contributions
3. sinking-fund contributions that are not hard-required
4. discretionary spending allowances

Never reduce required bill amounts or minimum debt payments automatically.

### Step G: Surface unresolved shortage

If no valid plan keeps required obligations and buffer safe:

Return:

- first shortage date
- shortage amount
- affected paycheck interval
- required obligations causing pressure
- optional items already reduced
- possible user-controlled options

Do not conceal the shortage by allowing a required bill to become late.

## 14. Manual bill move

When user moves a bill to another paycheck:

1. Validate eligibility.
2. Simulate the new plan.
3. Show:
   - current paycheck impact
   - destination paycheck impact
   - safe-to-spend changes
   - buffer status
   - lateness/window warnings
4. Require confirmation if warning exists.
5. Save manual allocation/lock as requested.
6. Recalculate.

## 15. Split payments/funding

A bill occurrence may be allocated across multiple paychecks.

Rules:

- sum of allocations may not exceed remaining required amount without explicit extra-payment semantics
- allocations must reference eligible funding sources
- full required amount must be funded by due date
- partial real payment reduces remaining amount
- schedule displays funded vs paid separately

Example:

```text
Credit Card required: $600
Aug 14 allocation: $300
Aug 21 allocation: $300
Funded: $600
Paid so far: $300
Remaining to pay: $300
```

Funding and payment are distinct concepts.

## 16. Marking a bill paid

When a payment is recorded:

1. Validate amount > 0.
2. Add payment row.
3. Update occurrence paid total.
4. If paid total >= required amount, mark occurrence Paid.
5. Otherwise mark Partial.
6. Update book balance if the payment uses an account tracked in the app.
7. Remove or reduce corresponding reservation.
8. Recalculate projection and future schedule.
9. Record activity.

Prevent duplicate ordinary payment actions after fully paid.

## 17. Early payment

If a bill is paid earlier than planned:

- update balance
- remove reservation
- rebuild future projection
- only move other bills if doing so materially resolves/improves a plan issue
- do not churn allocations unnecessarily

Use a stability preference: keep existing valid assignments unless a change is needed or materially improves buffer safety.

## 18. Paycheck received higher than expected

On actual paycheck > effective prior estimate:

- record actual amount
- calculate surplus difference
- recalculate plan
- leave incremental surplus uncommitted by default
- deterministic recommendation engine may show options
- user decides whether to commit surplus

Do not auto-spend it.

## 19. Paycheck received lower than expected

On actual paycheck < effective prior estimate:

1. record actual
2. recalculate
3. protect required obligations
4. rebalance movable bills
5. reduce optional commitments according to configured order
6. set safe-to-spend to zero if necessary
7. surface unresolved shortage

## 20. Savings

Savings goals and sinking funds have planned contributions.

Contributions may be:

- required
- optional

Default: optional.

Optional savings may be reduced to protect essential bills and buffer.

Track goal balance through explicit contributions/adjustments.

## 21. Debt

Debt record:

- balance
- APR
- minimum payment
- due day/date
- planned payment
- optional extra payment

Minimum payment behaves as required bill.

Extra payment behaves as optional.

### Comparison calculations

Support:

- Snowball: smallest balance first
- Avalanche: highest APR first
- Custom ordering

Simulation should return:

- projected payoff date
- estimated interest paid
- sequence

Use deterministic amortization calculations.

Clearly label estimates.

## 22. Spending allowances and budgets

Budget categories help planning but are not bank-synced.

Category budgets may inform planned discretionary allowances.

Actual manual transactions reduce balances and category remaining amounts.

Do not double-count a budget as both a reservation and an actual expense.

Define one consistent model:

- planned allowance is reserved/headroom planning
- actual spending consumes that allowance
- unused allowance remains available unless user chooses otherwise

## 23. Balance reconciliation

Input:

- account
- app balance
- actual entered balance

Compute:

```text
difference = actual_balance - app_balance
```

If difference is nonzero, user chooses:

- untracked spending/income
- balance adjustment
- cancel

Persist reconciliation record and balancing transaction/adjustment.

Recalculate plan afterward.

## 24. Safe-to-spend warning levels

Use financial state, not arbitrary color alone.

Suggested states:

- Healthy: projected minimum comfortably above buffer
- Tight: projected minimum is above buffer but within configurable small headroom
- Shortage: projected minimum below buffer

Default "tight" threshold can be `$100` above buffer or a user-configured value.

Do not call something a shortage when it is merely close.

## 25. Explainability

Every automatic scheduler move should have structured reason codes, e.g.:

- `LATEST_ELIGIBLE_PAYCHECK`
- `MOVED_EARLIER_TO_PROTECT_BUFFER`
- `AUTOPAY_FIXED_DATE`
- `MANUAL_LOCK`
- `PAYMENT_WINDOW_LIMIT`
- `ESSENTIAL_PRIORITY`
- `OPTIONAL_SAVINGS_REDUCED`
- `SHORTAGE_UNRESOLVED`

Store enough inputs to reconstruct a human explanation.

The AI may paraphrase explanations, but the structured reason and numbers come from the engine.

## 26. Stability

Avoid schedule thrashing.

When recalculating:

- preserve existing valid manual assignments
- preserve existing valid automatic assignments when they remain healthy
- change only what is necessary to respond to new data or materially improve safety

The app should not constantly rearrange bills for tiny differences.

## 27. Simulation

Provide a pure what-if function for:

- paycheck amount change
- bill amount change
- moving a bill
- one-time expense
- optional extra debt payment

Simulation must not persist until explicitly applied.

AI uses simulation for proposals.
