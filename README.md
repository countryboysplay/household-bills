# Household Bills

**Release candidate:** 1.0.0

Household Bills is a local Windows desktop application for planning bills around household paycheck schedules. All household financial data is stored locally in SQLite.

For a Windows installer build, run `scripts\build-release.ps1` and then follow `RELEASE_TEST_CHECKLIST.md`.


Local Windows household bill and paycheck planner.

## Current build

**Version 1.0.0 release candidate** is implemented in source. Phase 5 functionality has passed interactive testing and the project is now hardened for the NSIS installer test.

### Application foundation
- Tauri 2 desktop shell
- React + TypeScript + Vite frontend
- Rust-owned SQLite using bundled SQLite
- automatic versioned migrations
- local application data and backups
- household onboarding
- NSIS Windows installer configuration
- PowerShell setup, development, test, and installer scripts

### Core household planning
- recurring weekly, biweekly, semimonthly, and monthly paycheck schedules
- projected, expected, and actual paycheck amounts
- recurring and one-time bills
- variable-bill estimation
- one protected cash-buffer floor
- cross-paycheck bill funding/reservation
- autopay funding versus actual draft date
- chronological cash-flow projection
- shortage warnings
- **What to Pay** instructions showing bill, amount, recommended payment date, due date, and funding paycheck(s)
- non-splittable bills remain one provider payment even when multiple paychecks fund them
- bills that explicitly allow partial provider payments can show multiple recommended payment actions

### Working screens
- Dashboard
- Paycheck Planner
- Bills and payment history
- Spending & Balance Reconciliation
- Calendar
- Savings & Debt
- History
- Reports
- Settings
- Backup & Restore

### Privacy and scope
- no bank connection
- no cloud backend
- no AI
- no automatic money movement
- data remains local unless the user explicitly exports a report or copies a backup

## Windows development

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\scripts\setup-dev.ps1
.\scripts\dev.ps1
```

## Test checkpoint

```powershell
.\scripts\test.ps1
```

This runs the frontend TypeScript check, Tauri command permission checks, and Rust tests.

See `PHASE5_WINDOWS_TEST.md` for the functional acceptance test.

## Build `.exe` installer

Build the 1.0.0 release candidate installer with:

```powershell
.\scripts\build-release.ps1
```

The NSIS installer is produced under:

`src-tauri\target\release\bundle\nsis\`

## Financial integrity

The protected buffer is a balance floor, not an expense. Required financial calculations and scheduling decisions are owned by Rust. React displays those results and submits user actions but does not reproduce authoritative scheduling math.
## 1.0.1 GitHub updater bootstrap

Version 1.0.1 adds signed stable updates through GitHub Releases. Before building 1.0.1, follow **GITHUB_SETUP.md** exactly to create the public repository, generate the permanent Tauri updater signing key, configure the updater endpoint/public key, and add the GitHub Actions secrets.

The installed 1.0.0 build must be upgraded to 1.0.1 manually once. After that, future stable versions can be offered inside the app.

