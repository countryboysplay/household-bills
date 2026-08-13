# Build Phases

## General rule

Do not build every feature at once.

Each phase should leave the repository compiling and tests passing.

Maintain `BUILD_STATUS.md`.

## Phase 0 — Repository and design review

### Tasks

- Read specs.
- Inspect design references.
- Scaffold Tauri 2 + React + TypeScript + Vite.
- Establish coding standards.
- Establish test tooling.
- Add app icon placeholder.
- Create `BUILD_STATUS.md`.

### Exit criteria

- Development app launches.
- Production build starts successfully.
- Basic navigation shell renders.
- Rust and frontend tests/lint framework are working.

## Phase 1 — Local data foundation and installer skeleton

### Tasks

- SQLite connection in Rust.
- migration system.
- user/profile tables.
- household settings.
- account table.
- local app-data directories.
- initial onboarding state.
- backup service skeleton.
- NSIS build configuration.
- scripts:
  - `scripts/setup-dev.ps1`
  - `scripts/dev.ps1`
  - `scripts/test.ps1`
  - `scripts/build-installer.ps1`

### UI

- sidebar shell
- placeholder screens
- first-run wizard shell
- Settings basics

### Exit criteria

- Clean DB created automatically.
- Migrations run automatically.
- Profile/account settings persist.
- NSIS installer can be built.
- Installed app launches without developer runtime interaction.

## Phase 2 — Deterministic financial engine

Build before polishing all UI.

### Tasks

- Money value type.
- date/recurrence utilities.
- paycheck amount precedence.
- bill amount estimation.
- occurrence generation.
- projection engine.
- protected buffer.
- safe-to-spend.
- bill eligibility.
- automatic allocation.
- autopay handling.
- shortage detection.
- optional commitment reduction.
- reason codes.
- simulation engine.

### Tests

Extensive unit tests.

### Exit criteria

- Engine passes cases in `ACCEPTANCE_TESTS.md`.
- Same input produces same output.
- No AI code is involved.

## Phase 3 — Paychecks, Bills, Payments

### Tasks

- paycheck CRUD
- recurring paycheck generation
- variable paycheck amount entry
- bills CRUD
- bill occurrence generation
- bill allocations
- mark paid
- partial payments
- manual bill move
- lock
- autopay reserve/confirm
- activity records

### UI

Implement approved:

- Paycheck Planner
- Bills
- Add/Edit Bill
- Bill Detail

### Exit criteria

A household can enter paychecks and bills and get a working paycheck plan.

## Phase 4 — Dashboard, Calendar, Reconciliation

### Tasks

- dashboard aggregation
- balance projection
- recent activity
- calendar queries
- manual transactions
- balance reconciliation
- budgets/allowances basics

### UI

Implement approved:

- Dashboard
- Calendar
- Spending & Balance Reconciliation

### Exit criteria

User can operate daily household workflow from the app.

## Phase 5 — Savings, Debt, History, Backup

### Tasks

- savings goals
- sinking funds
- debt records
- snowball/avalanche/custom simulation
- history filters
- backup creation
- retention
- restore
- pre-migration backup
- optional attachments if core is stable

### UI

Implement:

- Savings & Debt approved reference
- History
- Backup Settings

### Exit criteria

Core non-AI product is release-candidate quality.

## Phase 6 — Installer and release hardening

AI has been removed from the roadmap. After Phase 5 validation, move directly to installer/release work.

### Tasks

- clean Windows install testing
- upgrade/migration testing
- uninstall behavior
- backup/restore disaster test
- crash/restart cases
- performance
- accessibility
- polish
- version metadata
- logs
- optional updater
- optional code signing pipeline placeholder

### Exit criteria

On a clean Windows 11 Home system:

1. run `.exe`
2. install
3. launch
4. complete onboarding
5. use core product
6. uninstall/reinstall without unexpected data loss

## Phase 7 — Final UX pass

Compare every screen to approved design references.

Correct:

- spacing
- typography
- sidebar
- cards
- colors
- tables
- dialogs
- empty states
- warning hierarchy
- loading states

Do not change financial behavior during a cosmetic pass without updating tests.
