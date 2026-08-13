# Phase 2 Windows Test

Phase 1 has already compiled and launched successfully on Windows 11.

## 1. Test the financial engine

Open PowerShell in the project folder:

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\scripts\test.ps1
```

Expected outcome:

- frontend TypeScript check completes
- Cargo compiles the Rust test target
- all Money/domain/scheduler/recurrence tests pass
- final line: `All tests passed.`

Compiler warnings are acceptable at this checkpoint. Errors or failing tests are not.

## 2. Launch the application

```powershell
.\scripts\dev.ps1
```

Expected outcome:

- Vite starts
- Rust/Tauri builds
- `household-bills.exe` runs
- the existing Phase 1 UI opens normally
- existing local onboarding data remains intact if this folder/version points to the same app identifier/data directory

## 3. What to send back if anything fails

Send a screenshot or paste the console output beginning at the first `error:` line. There is no need to include the dependency download/compile lines above the first error.

## Why the UI looks mostly unchanged in Phase 2

Phase 2 intentionally builds and validates the authoritative financial engine before Phase 3 connects it to the approved Paycheck Planner and Bills screens.
