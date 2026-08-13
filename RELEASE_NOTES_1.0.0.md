# Household Bills 1.0.0

First installable release candidate of the local Household Bills desktop application.

## Included

- recurring weekly, biweekly, twice-monthly, monthly, and manual paychecks
- individual paycheck amount overrides
- recurring and one-time bills
- deterministic paycheck-to-bill funding
- explicit What to Pay guidance with due date and recommended payment date
- multi-paycheck reservation for a single provider payment
- optional partial provider payments
- autopay reservation/draft guidance
- protected cash buffer and safe-to-spend calculation
- balance reconciliation and manual transactions
- savings goals, sinking funds, and debt tracking
- calendar, history, and reports with CSV export
- automatic local backups and restore workflow
- 30/60/90-day Paycheck Planner views
- all financial data stored locally in SQLite

## Release safety

- Existing data is stored outside the application installation directory.
- A backup is created automatically before any pending database migration is applied.
- Restore candidates are SQLite integrity-checked before being accepted.
- Normal application use does not require an internet connection.
- No bank connection or cloud account is used.
