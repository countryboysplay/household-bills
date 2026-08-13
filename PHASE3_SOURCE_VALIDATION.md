# Phase 3 Source Validation

Validation performed in the build environment before Windows handoff:

- Phase 1 and Phase 2 source were used as the validated baseline.
- `001_initial.sql` + `002_phase3.sql` execute successfully against a clean SQLite database.
- Phase 3 migration adds bill due-day/payment-window fields, paycheck posting state, and source-linked transactions.
- The React/TypeScript application source passes a strict app-level TypeScript validation using temporary local module shims because npm package installation is unavailable in this build environment. The shims are removed from the delivered source.
- JSON/TOML configuration remains structurally valid.
- Phase 3 adds Rust database-integration tests that will run as part of `scripts/test.ps1` on Windows.

The final Rust/Tauri compile for the new Phase 3 code must be validated on the user's Windows development machine, because this environment does not contain the Windows Rust/MSVC toolchain. `scripts/test.ps1` is configured to stop on any frontend or Rust failure.
