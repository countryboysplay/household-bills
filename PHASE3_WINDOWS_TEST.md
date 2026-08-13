# Phase 3 Windows Validation

Phase 3 connects the tested scheduler to SQLite and adds the first real working product screens: Bills and Paycheck Planner.

## 1. Extract to a new folder
Do not overwrite the validated Phase 2 source folder.

## 2. Run the automated checks
```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\scripts\test.ps1
```
A real pass must show the frontend type check succeeding, Rust tests executing, and the final line:

`All frontend and Rust tests passed.`

## 3. Launch
```powershell
.\scripts\dev.ps1
```
The same local household database from Phase 2 will be upgraded automatically with migration `002_phase3`.

## 4. Functional smoke test
1. Open **Bills**.
2. Add a monthly fixed bill, for example Internet, $90, due on the 22nd.
3. Add a variable bill, for example Electric, estimated $184.22, due on the 18th.
4. Open **Paycheck Planner**.
5. Add at least two future paychecks for the household.
6. Confirm the bills are assigned to paycheck cards after recalculation.
7. Change one paycheck amount and save it. Confirm the schedule recalculates.
8. Return to **Bills**, select a bill, and mark it paid.
9. Confirm the bill payment appears in payment history and the checking balance changes.
10. Close and relaunch the app. Confirm the entered bills/paychecks remain.

## Important behavior to verify
- The protected buffer is a balance floor, not a repeated $500 expense.
- A received paycheck is posted to the local checking balance only once.
- Marking a bill paid decreases the local checking balance and removes that occurrence from future scheduling.
- No bank connection is used. All data stays in local SQLite.
