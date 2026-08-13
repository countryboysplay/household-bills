# Phase 3 Implementation

Phase 3 makes the scheduler database-backed and exposes the first production workflows.

## Added
- Database migration `002_phase3`.
- Real bill templates and rolling bill occurrences.
- Fixed and variable bill amounts.
- Monthly and one-time bills.
- Manual and autopay bill types.
- Bill priority, responsibility, split-payment eligibility, and payment windows.
- Payment history and partial/full payment recording.
- Manual paycheck entry with projected, expected, and actual amount precedence.
- Received-paycheck posting to the primary local account.
- SQLite-backed schedule recalculation and allocation persistence.
- Real Bills screen based on the approved design.
- Real Paycheck Planner screen based on the approved design.
- Phase 3 database integration tests.

## Still intentionally deferred
- Recurring paycheck rule editor.
- Drag/drop bill movement and permanent paycheck locks in the GUI.
- Full Dashboard data wiring.
- Spending/reconciliation UI.
- Savings/debt UI.
- Calendar UI.
- Local AI runtime.
- Production installer/update workflow.
