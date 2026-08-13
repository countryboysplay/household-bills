# Architecture

> **Phase 5 scope override (current):** AI has been removed from the Household Bills roadmap. Any AI, llama.cpp, model, Ask AI, or AI proposal material later in this historical document is superseded by `AI_SPEC.md` and must not be implemented. The product remains deterministic and local.

## 1. Architectural objective

Build a self-contained Windows desktop application with a polished React UI and a Rust core that owns persistence, calculations, scheduling, backups, and AI integration boundaries.

The application is single-machine and local-first.

## 2. High-level architecture

```text
┌──────────────────────────────────────────────────────────────┐
│                      Tauri Desktop App                       │
│                                                              │
│  React + TypeScript UI                                       │
│          │                                                   │
│          │ typed Tauri commands/events                       │
│          ▼                                                   │
│  Rust Application Layer                                     │
│   ├── Commands / DTOs                                        │
│   ├── Domain Services                                        │
│   ├── Deterministic Scheduler                                │
│   ├── Projection / Reporting                                 │
│   ├── Backup / Restore                                       │
│   ├── AI Gateway                                             │
│   └── SQLite Repository Layer                                │
│          │                                                   │
│          ▼                                                   │
│      SQLite database                                         │
│                                                              │
│  Optional llama.cpp sidecar                                  │
│      127.0.0.1 only                                          │
└──────────────────────────────────────────────────────────────┘
```

## 3. Frontend

Use:

- React
- TypeScript
- Vite
- a small, maintainable component system
- a charting library appropriate for desktop charts
- accessible form controls

Avoid putting financial business logic in React components.

Frontend responsibilities:

- rendering
- input collection
- form validation that improves UX
- state/query caching
- navigation
- optimistic UI only where safe
- displaying authoritative results returned from Rust

Frontend must not open SQLite directly.

## 4. Rust application layer

Rust owns:

- database connection and migrations
- repositories
- transaction boundaries
- scheduler
- projections
- safe-to-spend
- bill estimates
- debt strategy calculations
- backup and restore
- attachment storage if implemented
- local AI process lifecycle
- AI context/tool dispatcher
- validation of any proposed change

Expose focused Tauri commands such as:

```text
get_dashboard_summary
list_paychecks
update_paycheck_occurrence
list_bills
get_bill_detail
create_bill
update_bill
mark_bill_paid
record_partial_payment
reassign_bill_allocation
reconcile_account
list_transactions
create_transaction
get_calendar_month
get_savings_debt_summary
run_scheduler
simulate_change
create_backup
restore_backup
ask_local_ai
apply_validated_proposal
```

Exact naming may vary. Keep commands typed and narrow.

## 5. Persistence

Use SQLite stored in the application's user data directory.

Recommended approach:

- Rust SQLite library such as `sqlx` or another mature Rust SQLite crate
- versioned SQL migrations
- foreign keys enabled
- WAL mode where appropriate
- transactions for multi-step writes
- `DECIMAL` monetary values represented as integer cents in application/domain storage

### Money representation

Do not use binary floating point for authoritative money calculations.

Preferred domain representation:

```text
Money(i64 cents)
```

Examples:

- `$10.00` = `1000`
- `-$45.21` = `-4521`

APR/percentages may use integer basis points or a decimal library.

## 6. Time and dates

Store:

- dates that are conceptually dates as ISO `YYYY-MM-DD`
- timestamps in UTC
- local display in the user's Windows timezone

Payday and due-date calculations are date-based.

Avoid accidental timezone shifts for bill due dates.

## 7. Domain boundaries

Suggested modules:

### `domain`
Entities/value objects:

- User
- Account
- Money
- BillTemplate
- BillOccurrence
- BillAllocation
- Payment
- IncomeSource
- PaycheckOccurrence
- Transaction
- SavingsGoal
- Debt
- Reconciliation
- ProtectedBuffer
- ScheduleDecision

### `scheduler`
Pure or near-pure financial planning logic.

It should accept a deterministic input snapshot and return a plan/result without directly mutating the UI.

### `projection`
Builds chronological projected account balances and safe-to-spend values.

### `db`
Repositories and migrations.

### `services`
Application use cases that coordinate persistence and domain logic.

### `ai`
Local model process, structured context, response validation, proposal generation.

### `backup`
Consistent backup/restore and retention.

## 8. Scheduler purity

Where practical, the scheduler should be testable without a live database.

Example:

```text
ScheduleInput
  -> scheduler::build_plan(...)
  -> ScheduleResult
```

The service layer loads data from SQLite, creates `ScheduleInput`, invokes the scheduler, validates the result, and persists generated allocations/decisions.

This separation is critical for trustworthy tests.

## 9. Events

Use Tauri events sparingly for cross-screen refreshes.

Example events:

- `household-data-changed`
- `schedule-recalculated`
- `backup-created`
- `ai-status-changed`

A successful write should trigger data invalidation/refetch rather than duplicating business rules in frontend state.

## 10. Local AI sidecar

Bundle/provision `llama-server.exe` and required runtime files as Tauri sidecar resources.

Requirements:

- start only when AI is enabled/needed
- bind to `127.0.0.1`
- use a non-public local port
- stop when application exits
- handle startup failure gracefully
- CPU fallback is acceptable
- app remains fully functional if AI fails
- never expose SQLite credentials/path to the model unnecessarily
- never allow raw model output to directly execute a database mutation

Use an app-controlled structured protocol.

Do not depend on fragile free-form parsing where schema-constrained output is available.

## 11. Model storage

Do not commit multi-gigabyte model files to Git.

Store model files under an app-managed model directory.

The first-run AI setup may:

- detect compatible GPU
- offer default model
- download with visible progress
- verify SHA-256/checksum
- record installed model metadata
- allow AI to remain disabled if download is skipped

An optional offline installer variant may bundle the model later.

## 12. Backups

A backup must be a consistent snapshot.

For SQLite, do not merely copy an actively written WAL database without a safe backup process.

Use SQLite backup APIs or a controlled checkpoint/backup routine.

Backup bundle should contain:

- SQLite snapshot
- manifest/version metadata
- optional attachments if attachments are enabled

Do not include the local LLM model in ordinary backups.

## 13. Installer

Build Windows NSIS installer with Tauri.

Target normal per-user installation unless a technical reason requires otherwise.

The installer should include:

- app executable
- frontend assets
- Rust runtime code
- SQLite capability embedded in app
- AI runtime binaries if AI feature ships enabled
- required app resources
- app icon
- Start Menu shortcut
- optional desktop shortcut

The installer should not require end users to separately install build tools.

## 14. Upgrade strategy

Every release must:

1. Detect existing application data.
2. Create a pre-upgrade backup.
3. Apply forward database migrations.
4. Preserve user data.
5. Fail safely if migration fails.
6. Avoid destructive schema changes without a migration/restore path.

## 15. Error handling

User-facing errors should be understandable.

Example:

Bad:
`SQLITE_CONSTRAINT_FOREIGNKEY`

Good:
`This bill could not be deleted because it has payment history. Archive the bill instead.`

Log technical details locally for troubleshooting.

## 16. Security boundary

Treat frontend input as untrusted even though the app is local.

Validate:

- money ranges
- dates
- recurrence rules
- IDs
- allocation amounts
- payment amounts
- restore files
- AI proposal schemas

Rust should enforce invariants.

## 17. Suggested repository structure

```text
src/
  app/
  components/
  features/
    dashboard/
    paychecks/
    bills/
    calendar/
    spending/
    savings-debt/
    history/
    ai/
    settings/
  lib/
  styles/
  types/

src-tauri/
  src/
    main.rs
    commands/
    db/
    domain/
    scheduler/
    projection/
    services/
    backup/
    ai/
  migrations/
  binaries/
  resources/

docs/
design_references/
scripts/
tests/
```

## 18. No hidden server architecture

Do not add:

- FastAPI
- Express backend
- Node server
- PostgreSQL server
- Docker Compose
- Caddy
- localhost web application server for core app

The Tauri/Rust process is the local application backend.

The only optional local HTTP service is the loopback-only LLM sidecar.
