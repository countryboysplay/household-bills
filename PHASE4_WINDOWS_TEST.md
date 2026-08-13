# Household Bills Phase 4 Windows Test

Phase 3 is considered validated. Phase 4 adds real Dashboard, Spending & Balance Reconciliation, Calendar, and History screens.

## 1. Automated gate

From the extracted project folder:

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\scripts\test.ps1
```

Do not continue if there are red errors. The final line should be:

```text
All frontend and Rust tests passed.
```

## 2. Launch

```powershell
.\scripts\dev.ps1
```

Your existing Phase 3 database should be reused automatically.

## 3. Reconcile the primary checking balance

1. Open **Spending**.
2. Click **Reconcile Balance**.
3. Enter the actual current balance you see in your checking account. For testing, a fake balance is fine.
4. Confirm the app shows the difference before you save.
5. Save the reconciliation.
6. Verify the account card changes to the entered balance.
7. Return to **Dashboard** and verify Current Cash and Safe to Spend change.
8. Return to **Paycheck Planner** and verify the negative-current-cash warnings respond to the new balance.

Expected behavior:
- A negative difference is recorded as untracked spending.
- A positive difference is recorded as a reconciliation adjustment.
- The account balance becomes exactly the value entered.
- The scheduler recalculates after reconciliation.

## 4. Manual spending transaction

1. Open **Spending**.
2. Click **Add Transaction**.
3. Add an expense such as `Groceries`, `$75.00`, today.
4. Confirm the account balance drops by exactly $75.00.
5. Confirm the transaction appears in the list and monthly spending summary.
6. Add a small manual income item and verify the inverse behavior.

## 5. Dashboard

Verify the Dashboard is no longer populated by demonstration data.

Check:
- Current Cash matches the manually tracked accounts.
- Next Paycheck is one of your real recurring paycheck occurrences.
- Safe to Spend changes after reconciliation or a transaction.
- Upcoming Bills shows your real bills.
- Cash Flow uses recorded transactions.
- Paycheck Overview matches the Planner.
- Recent Activity reflects actions you actually performed.
- Alerts correspond to the real planner/account state.

## 6. Calendar

1. Open **Calendar**.
2. Navigate to a month containing your mortgage and paycheck schedules.
3. Confirm paycheck dates match the Planner.
4. Confirm the Mortgage appears on its actual due date.
5. Select the Mortgage date and verify Day Details preserves the real due date and separately shows an earlier pay-by date when applicable.
6. Change months and return. No duplicate events should appear.

## 7. History

Open **History** and verify:
- Activity includes bill/payment/paycheck/reconciliation actions.
- Payments includes bills you marked paid.
- Reconciliations includes the balance adjustment from step 3.
- Search works on the active tab.

## 8. Persistence

Close Household Bills completely and reopen it.

Verify:
- Reconciled account balance remains.
- Manual transactions remain.
- Dashboard totals remain consistent.
- Calendar events remain correct.
- History records remain.
- Existing bills, paid status, paycheck schedules, and planner allocations are still intact.

## Phase 4 pass criteria

Phase 4 is ready to proceed when all automated tests pass and the four new screens behave correctly through a full close/reopen cycle.
