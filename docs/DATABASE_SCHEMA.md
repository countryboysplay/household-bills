# Database Schema

## 1. Goals

The schema should:

- support two household profiles
- preserve historical bill/payment data
- support recurring templates plus concrete occurrences
- support split funding across paychecks
- support deterministic scheduling explanations
- make backup/restore simple
- avoid enterprise complexity

Use SQLite foreign keys.

Store money as integer cents.

Use migrations from day one.

## 2. Conventions

Suggested common fields:

```text
id              TEXT/UUID primary key
created_at      UTC timestamp
updated_at      UTC timestamp
archived_at     nullable UTC timestamp
```

UUIDs are optional; stable string IDs or integer IDs are acceptable if consistently implemented.

## 3. `users`

```text
id
display_name
pin_hash                nullable
avatar_key              nullable
theme_preference        system|light|dark
is_active
created_at
updated_at
```

Both users share the same household.

No complex roles in V1.

## 4. `household_settings`

One-row or key/value configuration.

Recommended typed columns where important:

```text
id
household_name
primary_account_id
protected_buffer_cents
tight_headroom_cents
default_planning_horizon_days
ai_enabled
created_at
updated_at
```

Additional preferences can live in a JSON/settings table if appropriate.

## 5. `accounts`

```text
id
name
account_type          checking|savings|cash|credit|other
book_balance_cents
is_primary_bill_account
is_active
created_at
updated_at
archived_at
```

Credit accounts may use negative balance display semantics, but store consistently.

## 6. `balance_reconciliations`

```text
id
account_id
app_balance_before_cents
actual_balance_cents
difference_cents
resolution_type       untracked_spending|untracked_income|adjustment
note
performed_by_user_id
performed_at
created_at
```

A reconciliation should also result in a ledger adjustment/transaction where appropriate.

## 7. `categories`

Keep simple.

```text
id
name
kind                  bill|spending|income|savings|debt|other
icon_key
is_system
is_active
sort_order
```

Ship with basic defaults.

Do not implement a complex tag system in V1.

## 8. `income_sources`

Recurring paycheck definition.

```text
id
user_id
name
schedule_type         weekly|biweekly|semimonthly|monthly|custom
schedule_config_json
default_projected_amount_cents
weekend_holiday_rule
is_active
created_at
updated_at
archived_at
```

`schedule_config_json` holds recurrence details cleanly.

## 9. `paycheck_occurrences`

Concrete paycheck event.

```text
id
income_source_id
pay_date
projected_amount_cents
expected_amount_cents       nullable
actual_amount_cents         nullable
status                      projected|updated|received|skipped
is_date_override
note
created_at
updated_at
```

Effective amount precedence is defined in `FINANCIAL_ENGINE.md`.

## 10. `bill_templates`

Recurring bill definition.

```text
id
name
category_id
amount_type                 fixed|variable
fixed_amount_cents          nullable
fallback_estimate_cents     nullable
estimate_window_count       default 6
recurrence_type             monthly|weekly|quarterly|annual|custom|none
recurrence_config_json
due_rule_json
payment_type                manual|autopay
priority                    essential|normal|flexible
payment_window_type         anytime|near_due|custom
payment_window_config_json
can_split                   boolean
assigned_user_id            nullable (null = Shared)
preferred_paycheck_rule_json nullable
is_active
notes
created_at
updated_at
archived_at
```

## 11. `bill_occurrences`

Concrete occurrence.

```text
id
bill_template_id            nullable for standalone one-time if desired
name_snapshot
category_id
due_date
latest_payment_date
earliest_payment_date
estimated_amount_cents
manual_amount_override_cents nullable
actual_required_amount_cents nullable
status                      upcoming|scheduled|partial|paid|late|skipped
payment_type_snapshot
priority_snapshot
is_one_time
is_due_date_override
scheduled_payment_date      nullable
lock_type                   none|occurrence|recurring_rule
lock_metadata_json          nullable
notes
created_at
updated_at
```

Snapshots preserve historical meaning if template changes later.

## 12. `bill_allocations`

Funding assignment from current cash/paychecks.

This table is required because a bill can be split.

```text
id
bill_occurrence_id
paycheck_occurrence_id      nullable
funding_source_type         current_cash|paycheck
allocated_amount_cents
source                      scheduler|manual
is_locked
reason_code
created_by_user_id          nullable for scheduler
created_at
updated_at
```

Constraint:

Total active allocations should not exceed the amount requiring funding unless explicitly supported as extra payment.

## 13. `payments`

Actual payment event.

```text
id
bill_occurrence_id
account_id                  nullable
amount_cents
paid_date
paid_by_user_id
payment_method              nullable
note                        nullable
created_at
updated_at
```

Occurrence status is derived/updated from payment total.

## 14. `transactions`

Manual household ledger/spending entries.

```text
id
account_id
transaction_date
description
category_id
amount_cents                signed
transaction_type            spending|income|adjustment|transfer_component|other
status                      entered|cleared
source                      manual|reconciliation|system
created_by_user_id
note
created_at
updated_at
```

Avoid duplicating a confirmed bill payment as an unrelated transaction. If the ledger requires a transaction row for a payment, link it explicitly or use one authoritative ledger mechanism.

Choose one consistent implementation.

## 15. `transfers`

If separate transfer object is used:

```text
id
from_account_id
to_account_id
amount_cents
transfer_date
created_by_user_id
note
created_at
```

Do not count transfers as income or spending in reports.

## 16. `budgets`

```text
id
category_id
period_type               monthly
amount_cents
is_active
created_at
updated_at
```

Keep budgeting basic.

## 17. `savings_goals`

Use one table for goals and sinking funds.

```text
id
name
goal_type                 savings|sinking_fund|emergency
target_amount_cents
target_date               nullable
current_amount_cents
planned_contribution_cents
contribution_frequency
is_required_contribution
is_active
created_at
updated_at
archived_at
```

## 18. `goal_contributions`

```text
id
goal_id
account_id                nullable
amount_cents
contribution_date
paycheck_occurrence_id    nullable
created_by_user_id
note
created_at
```

## 19. `debts`

```text
id
name
debt_type
balance_cents
apr_basis_points
minimum_payment_cents
due_day_or_rule_json
planned_payment_cents
extra_payment_cents
custom_priority
is_active
created_at
updated_at
archived_at
```

Minimum payments may generate bill occurrences or be linked to a bill template. Prefer a single source of truth to avoid double counting.

## 20. `schedule_decisions`

Stores explainability metadata for automatic decisions.

```text
id
scheduler_run_id
bill_occurrence_id        nullable
decision_type
reason_code
before_json               nullable
after_json                nullable
explanation_data_json
created_at
```

Do not store generated AI prose as the authoritative reason.

## 21. `scheduler_runs`

```text
id
trigger_type
trigger_entity_type       nullable
trigger_entity_id         nullable
planning_start_date
planning_end_date
input_hash
status                    success|warning|error
shortage_date             nullable
shortage_cents            nullable
created_at
```

Useful for diagnostics and explainability.

## 22. `activity_log`

Simple shared history.

```text
id
actor_user_id             nullable for system
entity_type
entity_id
action_type
summary
metadata_json             nullable
created_at
```

Examples:

- `bill_paid`
- `paycheck_updated`
- `bill_created`
- `bill_archived`
- `balance_reconciled`
- `schedule_override`

## 23. `attachments` — optional V1 / later phase

```text
id
bill_template_id          nullable
bill_occurrence_id        nullable
original_filename
stored_filename
relative_path
size_bytes
sha256
added_by_user_id
created_at
```

Never store large blobs directly in SQLite unless there is a compelling reason.

## 24. `ai_sessions` — optional

If chat history is persisted:

```text
id
user_id
title
created_at
updated_at
```

## 25. `ai_messages` — optional

```text
id
session_id
role
content
structured_metadata_json nullable
created_at
```

Do not treat AI conversation data as financial source of truth.

## 26. `app_meta`

```text
key
value
```

Examples:

- schema version
- first-run completed
- installation ID
- app version last opened

## 27. Indexes

At minimum index:

- `paycheck_occurrences(pay_date)`
- `bill_occurrences(due_date, status)`
- `payments(bill_occurrence_id, paid_date)`
- `transactions(account_id, transaction_date)`
- `activity_log(created_at)`
- `bill_allocations(bill_occurrence_id)`
- `schedule_decisions(bill_occurrence_id)`

## 28. Data integrity

Use constraints where practical:

- money amounts that must be positive
- enum checks
- valid foreign keys
- only one primary bill account
- allocation totals validated in Rust transaction
- payment totals validated in Rust transaction

## 29. Deletion

Prefer archive for bills, accounts, goals, and debts with history.

Do not cascade-delete historical payments casually.

Permanent deletion is appropriate only for unused records with no meaningful history or through an explicit cleanup action.

## 30. Migration rule

Never edit an already released migration.

Add a new forward migration.

Before production migration, create an app backup.

Migration failure must leave the previous user data recoverable.
