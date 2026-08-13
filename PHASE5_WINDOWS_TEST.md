# Phase 5 Windows Functional Test

Run this test before building the installer.

## A. Automated gate

From the Phase 5 project folder:

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\scripts\test.ps1
```

Do not proceed unless frontend type checking, Tauri permissions, and Rust tests all pass.

Then launch:

```powershell
.\scripts\dev.ps1
```

Your existing Phase 4 data should migrate automatically and remain intact.

## B. Core purpose test: What to pay and when

Use your existing paycheck schedules and at least two unpaid bills.

1. Open **Dashboard**.
2. Find **What to Pay**.
3. For each displayed bill verify it clearly shows:
   - bill name
   - amount remaining
   - recommended payment date (or autopay draft date)
   - actual due date
   - which paycheck(s) or current cash fund it
4. Open **Paycheck Planner** and verify **Payment Instructions** gives the same actual payment dates.
5. Verify the paycheck cards still show money being reserved from each paycheck.

### Large bill / multi-paycheck test

Use a bill larger than one paycheck, such as the mortgage test used in Phase 3.

Expected behavior:

- money may be reserved from multiple earlier paychecks
- all funding sources appear in What to Pay / Payment Instructions
- if partial payments to the provider are disabled, there is still only **one** recommended actual payment date
- the app must not tell you to make several provider payments merely because several paychecks funded the bill

### Intentional partial-payment test

Create a test bill with **Allow partial payments to the bill provider** enabled and make it large enough to require two paycheck buckets.

Expected behavior:

- What to Pay / Payment Instructions may show two separate provider payment dates and amounts
- the sum of those payment actions must equal the remaining bill amount when fully funded
- Calendar must show each recommended partial-payment action separately
- disabling partial payments should return the same funding pattern to one actual provider-payment date

## C. Autopay test

Add or use an autopay bill.

Expected:

- Dashboard says the bill will **draft** on its actual draft/due date
- funding may come from an earlier paycheck
- Paycheck Planner treats earlier money as reserved
- Calendar shows a recommended-payment/autopay action and the actual due event

## D. Mark paid

Mark a manual bill paid.

Expected:

- it disappears from What to Pay for that occurrence
- it disappears from unpaid payment instructions
- payment history remains
- next recurring occurrence remains scheduled normally

## E. Calendar

Open Calendar and verify:

- paycheck dates appear
- actual bill due dates appear
- recommended payment actions appear as separate amber events
- a bill whose due date shifts for planning still keeps its actual due date visible

## F. Savings & Debt

1. Add a savings goal with a small test planned contribution.
2. Confirm Paycheck Planner shows the optional savings commitment.
3. Record an actual savings contribution and verify current cash decreases by exactly that amount.
4. Add a debt with APR and a planned extra payment.
5. Verify snowball and avalanche comparison displays.
6. Record a small extra debt payment and verify the debt balance and checking balance both decrease correctly.

## G. Settings

1. Change protected buffer to a recognizable test value.
2. Save Settings.
3. Verify Paycheck Planner recalculates using the new buffer.
4. Restore the desired real buffer value afterward.

## H. Backup

1. Open Settings.
2. Click **Create Backup Now**.
3. Verify a new backup appears in Recent Backups.
4. Do not perform a restore unless you are comfortable testing it with the current database.

Optional restore test:

- create a backup
- add one obvious temporary transaction
- request restore of the backup
- close Household Bills completely
- reopen it
- the temporary transaction should be gone and the backed-up state restored

The app creates a safety backup of the pre-restore state before scheduling the restore.

## I. Reports

1. Open Reports.
2. Select a range containing your test activity.
3. Verify income, bills, spending, savings, debt payments, and net cash flow are reasonable.
4. Click **Export CSV**.
5. Verify the file path is reported and the CSV exists in the Exports folder shown in Settings.

## J. Persistence

Close Household Bills completely and reopen it.

Verify:

- paycheck schedules remain
- bills remain
- paid status remains
- savings/debt data remains
- settings remain
- What to Pay produces the same deterministic plan
- no bills, paychecks, contributions, or allocations duplicate themselves

## K. Paycheck Planner 30 / 60 / 90 day filter

1. Open **Paycheck Planner**.
2. Confirm **30 Days** is selected by default.
3. Verify paycheck cards and Payment Instructions only show items inside the next 30 days.
4. Select **60 Days** and confirm later paychecks/bills appear.
5. Select **90 Days** and confirm the full planning window appears.
6. Switch back to **30 Days** and confirm the view becomes focused again without changing any underlying schedules or bill assignments.
7. Confirm recurring **Pay Schedules** at the top remain visible in all three views.

If all sections pass, Phase 5 is ready for installer/release hardening.
