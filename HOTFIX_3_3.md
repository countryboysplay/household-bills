# Hotfix 3.3

This hotfix addresses issues found during the Phase 3 functional test of the Paycheck Planner.

## Changes

- Projection warnings now evaluate the end-of-day balance after all events on a date, instead of warning on temporary intermediate balances between same-day paychecks and bills.
- Planner warnings are displayed in dollars instead of raw integer cents.
- The current-cash card now clarifies that it is the tracked account balance before future paychecks.
- Creating a second non-skipped paycheck for the same person on the same date is blocked with a clear validation message.
- Existing paychecks can now be removed from the Update Paycheck dialog.
- Removing a received paycheck reverses its posted deposit from the tracked account and removes the generated paycheck transaction.
- Changing the person on an existing paycheck now updates its income source correctly.
- Added `delete_paycheck` to the Tauri command permissions and regression permission check.
- Added a scheduler regression test proving that multiple same-day paychecks do not create a false end-of-day shortage warning.

## Existing test data

If duplicate paychecks already exist from an earlier Phase 3 build, open each duplicate paycheck and use **Remove Paycheck**. New duplicates are prevented.

A negative Current Cash value is not automatically a scheduler error. It reflects the app's tracked account balance. If test payments were recorded against a zero or low starting balance, Current Cash can legitimately be negative. Balance reconciliation will be expanded in Phase 4.
