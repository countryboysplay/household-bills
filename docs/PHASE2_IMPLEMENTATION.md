# Phase 2 Implementation

## Goal

Phase 2 turns the Phase 1 scheduler scaffold into the authoritative deterministic financial engine.

The engine remains independent of AI and independent of React. It accepts structured inputs and returns a reproducible plan.

## Implemented in this phase

### Money and paycheck rules

- integer-cent Money operations only
- actual > expected > projected paycheck precedence remains authoritative
- protected buffer is a floor, never a transaction or repeated per-paycheck expense
- if starting cash is below the buffer, future income replenishes the one global buffer deficit before that income becomes allocatable

### Paycheck funding buckets

The planner creates:

1. a current-cash bucket
2. one bucket for each future paycheck

A bucket exposes the amount available above the protected buffer, required bill allocations, optional commitments, remaining headroom, and a Healthy/Tight/Shortage state.

Unused earlier income can be deliberately reserved for later bills by moving those bills into an earlier paycheck bucket.

### Bill allocation

- latest eligible paycheck is preferred
- current cash is eligible when the planning date is inside the bill's payment window
- valid existing assignments are preserved when they remain safe, which stabilizes schedules across unrelated edits
- locked assignments are never silently overridden
- when the latest paycheck lacks headroom, an eligible manual bill is moved to the latest earlier bucket that can safely hold it
- bills that allow splitting may be funded across multiple paychecks
- if a required non-splittable bill cannot fit anywhere, it remains assigned to the latest eligible bucket and an explicit shortage is returned rather than hiding the bill
- invalid locks and bills with no eligible funding source remain unresolved with explicit warnings

### Autopay

Autopay has two separate dates/concepts:

- funding paycheck: may be selected/rebalanced
- draft date: fixed and never moved by the scheduler

### Optional commitments

Phase 2 models reducible commitments separately from required bills.

Preservation order when cash is tight:

1. sinking funds
2. optional savings
3. extra debt payments

Therefore extra debt is reduced first, then optional savings. Required bills are allocated before optional commitments.

### Chronological cash-flow projection

The engine separately projects actual cash movement:

- starting balance
- paycheck deposits
- manual bill payments on their recommended payment dates
- autopay drafts on fixed draft dates
- effective optional contributions

Every projection point returns:

- balance after event
- safe-to-spend after event
- below-buffer warning when applicable
- negative-balance warning when applicable

### Recurrence/date utilities

- weekly pay dates
- biweekly pay dates
- semi-monthly pay dates
- monthly pay dates
- monthly bill due-date generation
- end-of-month day clamping
- prior/next/exact business-day rules
- caller-supplied holiday set
- payment windows relative to due date

### Simulation

- paycheck amount change simulation
- manual bill move simulation
- before/after bucket headroom
- target-window validation
- scenario warnings

## Important invariant

`protected_buffer` is never inserted into the projection as an outgoing event.

If checking contains $1,000 and the buffer is $500, safe-to-spend is $500. Two future paychecks do not cause another $1,000 of fictitious buffer expenses.

## Phase 2 tests

The Rust test suite now covers:

- integer-cent arithmetic
- paycheck amount precedence
- variable bill averaging
- buffer floor behavior
- one-time buffer deficit replenishment
- latest eligible paycheck
- moving bills earlier to protect paycheck headroom
- fixed autopay draft date
- stable existing assignments
- valid and invalid locks
- split funding
- optional commitment reduction order
- lower paycheck simulation
- chronological event ordering
- manual move simulation
- weekend/holiday handling
- month-end clamping
- biweekly and semi-monthly generation

## Phase 3 boundary

Phase 2 is an engine library. Phase 3 connects these rules to SQLite CRUD and the approved Bills/Paycheck Planner user interfaces.
