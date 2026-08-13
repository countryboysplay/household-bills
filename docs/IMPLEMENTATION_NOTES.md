# Implementation Notes and Guardrails

> **Phase 5 scope override (current):** AI has been removed from the Household Bills roadmap. Any AI, llama.cpp, model, Ask AI, or AI proposal material later in this historical document is superseded by `AI_SPEC.md` and must not be implemented. The product remains deterministic and local.

## 1. Keep Version 1 personal

The approved mockups look sophisticated, but do not interpret that as a requirement for enterprise infrastructure.

The application has two people, one household, one Windows machine.

Prefer direct, understandable code.

## 2. Generated mockup caveats

The screenshots are design references generated from product discussions.

Some example totals/dates may not reconcile.

Do not use screenshot numbers as test fixtures unless they are independently verified.

## 3. Buffer semantics

This is the most important correction.

The protected buffer is a target minimum balance.

It is not a line-item bill.

It is not "spent."

It should not be totaled as an expense across paychecks.

## 4. AI semantics

The phrase "Ask Household AI" is approved.

The phrase "Auto-Plan AI" is not.

Use:

- Auto-plan
- Recalculate
- Scheduler

for deterministic planning.

AI explains and proposes.

## 5. Data ownership

Rust owns authoritative domain behavior.

React should not reimplement:

- balance math
- bill estimate math
- debt amortization
- safe-to-spend
- scheduling
- shortage detection

## 6. Money formatting

Domain: integer cents.

UI: localized US currency.

Never round intermediate money calculations through JS floating point.

## 7. Performance

Dataset is small.

Do not prematurely optimize with distributed caches or message brokers.

SQLite queries + in-process Rust calculations are sufficient.

## 8. Concurrency

Single application instance is preferred for V1.

If multiple app windows are allowed, writes still go through the same Rust/database process.

Consider preventing multiple independent app processes from opening the same live DB unless locking behavior is explicitly handled.

## 9. Export

CSV/PDF export is useful but secondary.

Do not delay core release for elaborate report templates.

## 10. Attachments

If implemented:

- copy file into app-managed attachment folder
- sanitize stored filename
- retain original display name
- hash file
- open through safe OS mechanism

Do not build OCR in V1.

## 11. Holidays

A small built-in US federal holiday calendar is acceptable for date planning, but bill-specific behavior must remain overridable.

Do not claim a biller will process a payment on a particular business day unless the user configured it.

## 12. Errors

Every important calculation error should fail closed.

Example:

If an allocation cannot be validated, show the bill as needing attention rather than dropping it from the plan.

## 13. Testing philosophy

Financial-engine tests are more important than snapshot UI tests.

Use UI tests for:

- marking paid
- updating paycheck amount
- manual move warning
- reconciliation
- profile attribution
- restore confirmation
- AI proposal approval

## 14. Dependency versions

Do not hardcode versions from these docs.

At implementation time, use current stable versions compatible with Tauri 2 and document them in lockfiles.

Consult primary project docs for exact Tauri/llama.cpp behavior.
