# Phase 2 Hotfix 2.2

## Fixes

- Added `@types/node` to development dependencies.
- Added explicit Node typings to `tsconfig.node.json` so `vite.config.ts` can type-check `process.env.TAURI_DEV_HOST`.
- Improved `scripts/test.ps1` so it installs dependencies when `node_modules` exists but the newly required Node typings are missing.
- Preserves the Hotfix 2.1 Rust borrow-checker fix and native exit-code enforcement.

## Windows validation

Run:

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\scripts\test.ps1
```

A valid pass must show:

1. Frontend type check completes without TypeScript errors.
2. `cargo test` compiles and runs the Rust test suite.
3. Rust reports all tests passed.
4. Final line: `All frontend and Rust tests passed.`

Then run:

```powershell
.\scripts\dev.ps1
```
