# Project Specification

> **Phase 5 scope override (current):** AI has been removed from the Household Bills roadmap. Any AI, llama.cpp, model, Ask AI, or AI proposal material later in this historical document is superseded by `AI_SPEC.md` and must not be implemented. The product remains deterministic and local.

## 1. Product

**Name:** Household Bills  
**Primary platform:** Windows 11 Home desktop  
**Primary users:** Jonathan and Tiffany  
**Use case:** Personal household bill and paycheck planning

Household Bills is a local desktop application that helps a two-person household decide **which bills to pay from which paycheck**, track whether bills have been paid, keep a manually reconciled account balance, and show what money is actually safe to spend.

The product should feel closer to a polished consumer finance desktop application than an accounting package.

## 2. Core product promise

At any moment, Jonathan or Tiffany should be able to open the app and understand:

- the current manually maintained household balance
- upcoming paychecks
- bills that are due
- which paycheck is expected to fund each bill
- which bills are already paid
- how much money is reserved
- whether the protected cash buffer is safe
- how much is safe to spend
- whether a future shortage is projected

## 3. Core operating model

### Shared household

The household is one shared financial pool.

Jonathan and Tiffany have separate profiles, but both see the same household data.

Each important action records which profile performed it so the other person can see, for example:

- Jonathan marked Electric paid
- Tiffany updated Water
- Jonathan reconciled Checking

No complex permissions are required in Version 1.

### Manual financial data

The application does not connect to a bank.

Balances, paychecks, bills, payments, purchases, and reconciliation information are entered or confirmed manually.

The app should minimize data-entry burden by allowing quick balance reconciliation. If the app balance differs from the real bank balance, the difference can be recorded as untracked spending or an adjustment.

### Paycheck-first planning

Bills are organized around paychecks.

The scheduler should normally use the **latest eligible paycheck before a bill must be paid**, but it may move a bill earlier when necessary to prevent a cash crunch or preserve the configured cash buffer.

### Protected cash buffer

The household defines a fixed minimum balance, such as $500.

The scheduler attempts to keep projected cash above that amount.

If required bills make that impossible, the app does not hide the problem. It shows the projected shortage and the date it occurs.

The buffer is a **minimum balance floor**, not a separate transaction or an expense.

## 4. Paychecks

Support multiple income sources, with at least one associated with each household member.

A recurring paycheck schedule can generate future paycheck dates, but each individual paycheck must be editable.

Each paycheck occurrence can contain:

- projected amount
- manually entered expected amount
- actual amount
- payday
- owner
- status
- optional note

This is important because Jonathan's paycheck can change due to commission.

When a paycheck amount changes, the financial plan should immediately recalculate.

If a paycheck is higher than expected, the extra should remain uncommitted until the user chooses what to do.

If a paycheck is lower than expected, recalculate the plan and surface any shortage.

## 5. Bills

Bills may be:

- recurring
- one-time
- fixed amount
- variable amount
- manual payment
- autopay
- essential
- normal
- flexible
- splittable across paychecks
- assigned to Jonathan, Tiffany, or Shared

The most common bill-entry path should remain short.

Basic fields:

- name
- amount or estimate
- due date
- recurrence
- manual/autopay

Advanced fields should be available but visually secondary.

## 6. Bill payment tracking

Either household member can mark a bill occurrence paid.

The app records:

- actual amount
- payment date
- paid by
- payment method if entered
- optional note

A paid occurrence must immediately appear as paid to the other profile.

Do not allow a second ordinary "Mark Paid" action once an occurrence is already fully paid. Allow editing the payment record instead.

Support partial payments.

## 7. Variable bills

A variable bill uses an estimate until the actual amount is known.

Default deterministic estimate:

- use the average of the last six fully paid occurrences when six are available
- if fewer than six exist, average the available paid occurrences
- if no history exists, use the bill template's manually entered estimate

A user-entered estimate for a specific occurrence overrides the automatic estimate.

When an actual amount is entered, recalculate future cash flow.

## 8. Autopay

Autopay is treated as a fixed draft event.

The scheduler reserves for it but does not move its draft date.

The user later confirms the draft/actual amount manually.

The scheduler may move other manual bills around an autopay event, but not the autopay itself unless the user edits the bill configuration.

## 9. Manual schedule control

Users retain control.

Support:

- manual reassignment to another eligible paycheck
- one-occurrence locks
- recurring assignment preferences
- optional split funding
- manual override of suggested schedule

The app should show the financial impact before accepting a manual move that would create a shortage or violate the protected buffer.

Never silently override a user lock.

## 10. Spending and balance reconciliation

Support manual spending entries such as:

- groceries
- gas
- dining
- shopping
- household
- other

The user does not need to record every transaction.

Provide a prominent **Reconcile Balance** workflow:

1. Show app balance.
2. User enters actual real-world balance.
3. Show difference.
4. Offer to record difference as untracked spending or balance adjustment.
5. Save reconciliation.
6. Recalculate plan.

## 11. Savings and debt

Keep this useful but not enterprise-complex.

Support:

- savings goals
- sinking funds
- debts
- minimum payments
- optional extra payments
- simple snowball/avalanche/custom comparison

Required bills and minimum debt payments take priority over optional savings and extra debt payments.

Optional savings contributions may be reduced when needed to protect required bills.

## 12. Calendar

Show:

- paydays
- bill due dates
- recommended manual payment dates
- autopay dates
- one-time expenses

Month view is the primary calendar view.

Selecting a date should reveal that day's events and relevant paycheck/bill details.

## 13. History

Provide a simple history of:

- payments
- paycheck updates
- reconciliations
- bill changes
- manual transactions

Include filters by date and type.

History is not an enterprise audit product, but important changes should be traceable.

## 14. Local AI assistant

Local AI is optional.

It may:

- answer plain-language questions about household data
- explain why the deterministic scheduler made a choice
- summarize bill/payment/spending trends
- explain a projected shortage
- compare deterministic what-if results
- propose a change

It may not:

- directly change the database
- independently calculate authoritative balances
- independently decide that a bill was paid
- bypass financial validation
- silently apply an action

All financial facts supplied to the model come from deterministic application services.

## 15. Data location

All financial data remains local on the Windows PC.

No bank connection.

No cloud database.

No cloud AI.

No public server.

## 16. Installation

The released application should be installed through a normal Windows `.exe` installer.

The installer should configure everything needed for runtime use.

The end user should not manually install:

- Rust
- Node.js
- npm
- Python
- SQLite tools
- Docker
- WSL
- PostgreSQL

## 17. First-run setup

Keep onboarding short:

1. Welcome
2. Create Jonathan and Tiffany profiles
3. Create primary checking account and enter current balance
4. Set protected cash buffer
5. Add paycheck schedules
6. Add bills
7. Review first schedule
8. Finish

AI setup can be optional after core onboarding.

## 18. Product tone

The app should be calm and factual.

Warnings should explain:

- what happened
- the dollar amount involved
- the date of impact
- what deterministic options are available

Avoid judgmental financial language.
