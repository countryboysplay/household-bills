# Hotfix 3.6

Fixes the Rust compile failure introduced in Hotfix 3.5.

## Fix
- Removed `payment_date` and `reason_code` from `PaycheckDto`.
- Those fields are intentionally retained on `PlannerBillDto`, where they describe how a bill allocation is scheduled.
- No database migration is required. Existing household data is preserved.

## Windows validation
Run `scripts\test.ps1` first. If all frontend and Rust tests pass, run `scripts\dev.ps1`.
