# Source Validation

## Previously validated on Windows
- Tauri/Rust project compiles and launches on Windows 11.
- Phase 2 financial-engine tests passed.
- Phase 3 bill/paycheck/payment integration passed functional testing.
- Recurring weekly and biweekly paycheck generation was functionally tested.
- Multi-paycheck reservation for a single large non-splittable bill was functionally tested.
- Phase 4 Dashboard, Balance Reconciliation, Calendar, Spending, and History were reported working by the user.

## Phase 5 authoring-environment checks
- `package.json`, Cargo version, and Tauri version are aligned at 0.5.0 where applicable.
- TypeScript/TSX semantic validation passed with temporary local type declarations.
- SQLite migrations 001 through 005 execute successfully on a fresh database.
- The new `bill_allocations.recommended_payment_date` field is present for preserving provider-payment actions.
- Split-payment Calendar SQL returns distinct provider-payment dates and amounts.
- Phase 5 command registration and Tauri permissions are aligned.
- User-facing AI references were removed from the release-path UI.

## Windows is the authoritative gate

The authoring container does not have Cargo/MSVC. Run:

```powershell
.\scripts\test.ps1
```

Then launch:

```powershell
.\scripts\dev.ps1
```

Do not proceed to installer/release hardening until `PHASE5_WINDOWS_TEST.md` is complete.
