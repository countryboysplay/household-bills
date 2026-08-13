# Phase 2 Hotfix 2.1

Fixed after first Windows compiler run:

1. `simulate_paycheck_amount_change` no longer holds a mutable paycheck borrow while calling `build_plan`, resolving Rust E0502.
2. `scripts/test.ps1` now fails immediately when `npm`, the typecheck, or `cargo test` returns a non-zero exit code. It cannot print `All tests passed` after a failed native command.
3. Removed the unused `rusqlite::params` import reported by the compiler.

Run `./scripts/test.ps1` again on Windows. Only run `./scripts/dev.ps1` after the test script reaches the final green `All tests passed.` message with no preceding Rust errors.
