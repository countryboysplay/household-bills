# Version 1 Scope

> **Phase 5 scope override (current):** AI has been removed from the Household Bills roadmap. Any AI, llama.cpp, model, Ask AI, or AI proposal material later in this historical document is superseded by `AI_SPEC.md` and must not be implemented. The product remains deterministic and local.

## Guiding principle

This is a personal household application. The goal is to make paying bills easier, not to recreate Quicken, YNAB, a bank portal, or an enterprise accounting product.

If a feature does not materially help answer "what should we pay from this paycheck?" or keep the shared household view accurate, it should probably wait.

## Must have in Version 1

### Household
- Two shared household profiles: Jonathan and Tiffany
- Simple profile switching/login
- Shared household data
- "Paid by" and "changed by" attribution

### Accounts
- Manually maintained checking account
- Optional savings/cash/credit account records
- One primary bill-paying account
- Manual balance reconciliation
- Balance adjustment/untracked-spending handling

### Paychecks
- Recurring paycheck schedules
- Weekend/holiday override support
- Manual occurrence date override
- Projected amount
- Expected amount
- Actual amount
- Variable week-to-week paycheck entry
- Immediate rescheduling after amount changes

### Bills
- Recurring and one-time
- Fixed and variable amounts
- Manual and autopay
- Due dates
- Priority
- Optional bill owner
- Payment window
- Partial payments
- Split funding across paychecks
- Manual assignment override
- Locking
- Archive inactive bills

### Scheduler
- Latest-eligible-paycheck default
- Smart earlier reassignment
- Protected cash buffer
- Priority-aware shortage behavior
- Clear shortage warnings
- Explainable deterministic decisions
- Safe-to-spend calculation
- 30/60/90/180/365-day planning horizons, default 90

### Payments
- Mark paid
- Actual amount
- Paid date
- Paid by
- Optional method/note
- Partial payment support
- Shared paid status

### Spending
- Manual transaction entry
- Simple spending categories
- Balance reconciliation
- Untracked-spending adjustment
- Basic monthly category budget support

### Savings & Debt
- Savings goals
- Sinking funds
- Debt balances/APR/minimum payment
- Optional extra payment
- Snowball/avalanche/custom comparison

### Calendar
- Paychecks
- Bill due dates
- recommended payment dates
- autopay dates
- one-time expenses

### History
- Recent activity
- payment history
- paycheck edits
- reconciliations
- important bill edits

### Backup
- Automatic local backups
- manual backup
- restore
- configurable retention
- optional backup folder selection

### Installer
- NSIS `.exe`
- first-run database initialization
- shortcuts
- clean upgrade path
- no developer dependencies for end user

### Local AI
- Optional
- local-only
- read/explain/summarize/propose
- no direct DB writes
- explicit approval before applying proposals
- application fully usable without AI

## Nice to have if Phase 1–5 are stable

- Bill attachments such as PDF statements or receipts
- CSV import for initial bills/transactions
- PDF/CSV reporting
- optional native notifications
- application updater
- printable reports
- what-if scenario comparison

## Explicitly out of scope for Version 1

- Bank account connection
- Plaid
- Open Banking
- credit-card feed synchronization
- automatic transaction downloads
- bill-pay initiation
- moving real money
- check printing
- public web access
- LAN server mode
- multi-device sync
- cloud backup
- cloud database
- cloud AI
- mobile-specific UI
- iOS application
- Android application
- multi-household SaaS
- enterprise roles and permissions
- tax filing
- investment portfolio management
- cryptocurrency tracking
- AI deciding authoritative balances
- AI silently modifying data
- autonomous financial actions
- complex tagging taxonomy
- receipt OCR as a core feature

## Scope-control rule

When implementing a requested feature, prefer the simplest implementation that solves the household problem.

Do not add infrastructure "for future scale" unless it materially improves reliability today.
