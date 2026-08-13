# Hotfix 3.4 - Recurring Paycheck Schedules

This hotfix connects the Phase 2 paycheck recurrence engine to the Phase 3 Paycheck Planner UI and SQLite persistence.

## Added
- Pay Frequency on the Add Paycheck form:
  - One-Time / Manual
  - Weekly
  - Every 2 Weeks
  - Twice a Month
  - Monthly
- Weekly and biweekly schedules use a Next Pay Date as the anchor.
- Twice-monthly schedules use two days of the month.
- Monthly schedules use one day of the month.
- Weekend date behavior can use prior business day, next business day, or exact date.
- Recurring schedules automatically generate future paycheck occurrences.
- The planner shows a Pay Schedules section with each household member's cadence, normal amount, and next payday.
- Individual generated paychecks can still be updated with expected/actual amounts without changing the recurring schedule.
- Existing one-time/manual paychecks are preserved and take precedence on a date so schedules do not create duplicates.
- A generated paycheck can be date-overridden without the scheduler recreating the original date.

## Database
Migration 003 adds `scheduled_pay_date` to paycheck occurrences so a generated occurrence retains its original schedule date when an individual occurrence is overridden.

## Test checkpoint
1. Run `scripts/test.ps1`.
2. Open Paycheck Planner and choose `+ Paycheck`.
3. Create a Weekly or Every 2 Weeks schedule.
4. Confirm future paycheck cards are generated at the correct cadence.
5. Open one generated paycheck, change the Expected Amount, save, and confirm only that occurrence changes.
