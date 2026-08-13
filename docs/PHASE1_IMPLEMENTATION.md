# Phase 1 Implementation Notes

## Runtime model

Household Bills is a native Tauri 2 Windows desktop application. The React UI talks only to narrow Rust commands. Rust owns SQLite and financial calculations.

## Local storage

On first launch, Rust resolves Tauri's application data directory and creates:

- `household_bills.sqlite3`
- `Backups/`

SQLite is compiled through rusqlite's bundled feature so the final end user does not need to install SQLite.

## First-run flow

1. Application opens the local database.
2. Versioned SQL migrations run in a transaction.
3. System categories are seeded idempotently.
4. `get_app_bootstrap` reports whether onboarding is complete.
5. New installs show the household setup form.
6. Onboarding creates the primary checking account, household settings, and user profiles.
7. The dashboard then reads authoritative summary values from Rust.

## Money

All authoritative monetary values are integer cents in Rust and SQLite.

Examples:

- $184.22 = 18422
- $90.00 = 9000
- -$45.21 = -4521

The frontend only formats cents for display.

## Protected buffer

The buffer is a floor. It is never written as a transaction. Phase 1 already contains a unit test for this invariant.

## Browser preview

The React app can run in a normal browser for rapid visual iteration. In that mode only, `src/lib/backend.ts` supplies clearly isolated preview data. Tauri builds never use those preview values because commands are invoked through the local Rust backend.

## Security direction

The WebView has a restrictive CSP and only receives the minimum application permissions needed for the Phase 1 commands.

## AI

No model runtime is shipped in Phase 1. An AI module placeholder records the architectural boundary. Later local AI must be optional, local-only, and unable to perform direct database mutations.
