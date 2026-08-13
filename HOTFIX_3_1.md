# Phase 3 Hotfix 3.1

Fixes two Rust compile errors found during the first Windows Phase 3 test:

1. `ensure_template_occurrences` unit test now supplies the required `template_id` argument.
2. `ensure_all_occurrences` now materializes the `query_map` iterator into a local vector while the prepared statement is still alive, resolving Rust error E0597.

The frontend TypeScript type check was re-run after these changes and completed without errors.

## Windows validation

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\scripts\test.ps1
```

A valid pass must show the Rust test summary and finish with:

`All frontend and Rust tests passed.`

Then run:

```powershell
.\scripts\dev.ps1
```
