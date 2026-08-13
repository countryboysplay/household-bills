# Phase 3 Hotfix 3.2

## Fixed

- Added Tauri 2 ACL permissions for all Phase 3 frontend commands. The commands were registered in `generate_handler!` but were not included in the `household-default` capability set, so write operations such as `save_bill` were rejected at runtime with `not allowed. Command not found`.
- Added permissions for bill list/detail/save/payment/archive and paycheck list/save/planner/scheduler commands.
- Improved Paycheck Planner loading so its header and `+ Paycheck` action remain visible while data loads or if the backend returns an error.
- Improved Bills initial-load error handling so backend load failures are shown in the page instead of becoming an unhandled promise rejection.
- Version bumped to 0.3.1.

## Paycheck entry

Open **Paycheck Planner** from the left sidebar. Use the blue **+ Paycheck** button in the upper-right corner. The button now remains visible even while the planner data is loading.

## Regression protection

`test.ps1` now includes a Tauri ACL check for every frontend command used by Phases 1–3. This catches a command that is registered in Rust but accidentally omitted from the desktop capability before runtime testing.
