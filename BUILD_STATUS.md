# Household Bills Build Status

Current version: **1.0.0 Release Candidate**

## Validated functionality

- Phase 1 Windows/Tauri shell passed
- Phase 2 deterministic scheduling engine tests passed
- Phase 3 bills, payments, paychecks, recurrence, and cross-paycheck funding passed
- Phase 4 dashboard, spending/reconciliation, calendar, and history passed
- Phase 5 payment guidance, savings/debt, settings, backups, reports, and 30/60/90 planner filters passed

## Release candidate hardening

- NSIS current-user installer configuration
- version alignment at 1.0.0
- generic pre-migration safety backup
- safety backup on app-version upgrades even without a migration
- backup integrity validation before restore
- About section with local data-folder controls
- release build script with SHA256 manifest
- release install/upgrade/uninstall test checklist

Next checkpoint: build `Household Bills Setup 1.0.0.exe` on Windows and complete `RELEASE_TEST_CHECKLIST.md`.
