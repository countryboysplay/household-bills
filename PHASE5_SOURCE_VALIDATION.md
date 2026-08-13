# Phase 5 Source Validation

Performed before packaging the Windows test build:

- Version aligned to 0.5.0 in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`.
- TypeScript/TSX semantic check passed using temporary local React/Tauri declarations in the authoring environment.
- All frontend `invoke()` command names have matching Tauri permissions.
- All Phase 5 Rust commands are registered in the Tauri invoke handler.
- SQLite migrations 001 through 005 execute successfully on a fresh in-memory database.
- Phase 5 migration adds `bill_allocations.recommended_payment_date`.
- Scheduler persistence writes the recommended provider-payment date for every allocation.
- Split-payment Calendar SQL smoke test returns separate action dates and amounts.
- Non-splittable bills retain a single provider-payment action while allowing multi-paycheck funding.
- No user-facing AI references remain in the application source.
- The unused Bills `Import Bills` control was removed rather than shipping a nonfunctional action.

## Authoritative compile gate

The authoring environment does not include Rust/Cargo/MSVC. Windows remains the authoritative full build and test environment.

Run:

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\scripts\test.ps1
```

Then launch:

```powershell
.\scripts\dev.ps1
```

Complete `PHASE5_WINDOWS_TEST.md` before installer/release hardening.
