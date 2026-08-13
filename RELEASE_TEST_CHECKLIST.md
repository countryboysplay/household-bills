# Household Bills 1.0.0 Release Test Checklist

Use this checklist only after `scripts\build-release.ps1` finishes successfully.

## 1. Protect the current test database

1. Open Household Bills from the development build.
2. Go to Settings and click **Create Backup Now**.
3. Click **Open Backup Folder** and verify a new `.sqlite3` file exists.
4. Close the development app completely.

## 2. Install the release build

1. Open the `release` folder.
2. Run **Household Bills Setup 1.0.0.exe**.
3. Windows may show a SmartScreen warning because this personal-use build is not code-signed.
4. Complete the current-user installation.
5. Launch Household Bills from the Start Menu.

## 3. Upgrade/data-preservation test

Because the application identifier is unchanged, the installed build should open the same local database used by the development build. Verify:

- household members are present
- paycheck schedules are present
- bills and bill payment history are present
- reconciled balance is present
- Savings & Debt data is present
- What to Pay guidance still matches the schedule

## 4. Core functional smoke test

- Dashboard opens without an error banner.
- Paycheck Planner defaults to 30 Days and the 60/90 Day filters work.
- What to Pay shows bill amount, due date, recommended payment date, and funding paychecks.
- Mark one test bill paid and verify it leaves What to Pay for that occurrence.
- Change one future paycheck amount and verify the planner recalculates.
- Reconcile the primary account balance.
- Add one manual spending transaction.
- Calendar shows paychecks, due dates, and recommended payment events correctly.
- History shows the payment/reconciliation activity.
- Reports load and CSV export succeeds.

## 5. Backup and folder test

In Settings:

- Version shows 1.0.0.
- Open Data Folder works.
- Open Backup Folder works.
- Open Exports Folder works.
- Create Backup Now succeeds.
- A selected backup can be scheduled for restore. Do not complete a restore unless you intentionally want to test it.

## 6. Restart persistence test

1. Close Household Bills completely.
2. Reopen it from the Start Menu.
3. Verify the changes from the smoke test remain.
4. Recalculate once and verify no bills/paychecks duplicate.

## 7. Uninstall/reinstall preservation test

Only do this after making a fresh backup.

1. Uninstall Household Bills through Windows Settings > Apps.
2. Verify the Household Bills data/backup directory still exists.
3. Reinstall `Household Bills Setup 1.0.0.exe`.
4. Launch it and verify the existing household data returns.

If all sections pass, the build can be treated as the Household Bills 1.0.0 release.
