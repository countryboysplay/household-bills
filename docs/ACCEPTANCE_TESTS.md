# Acceptance Tests

> **Phase 5 scope override (current):** AI has been removed from the Household Bills roadmap. Any AI, llama.cpp, model, Ask AI, or AI proposal material later in this historical document is superseded by `AI_SPEC.md` and must not be implemented. The product remains deterministic and local.

These are product-level acceptance cases. Claude should convert important cases into automated Rust tests and appropriate UI/integration tests.

Use integer cents.

## A. Money correctness

### A1 — cents

Input:

- bill = $184.22
- bill = $90.00

Expected total:

`$274.22`

No floating-point artifacts.

### A2 — negative transaction

Checking = $1,000.00  
Spending = $86.47

Expected:

`$913.53`

## B. Paycheck amount precedence

Projected = $2,175.00  
Expected = null  
Actual = null

Effective = $2,175.00

Then Expected = $2,347.18

Effective = $2,347.18

Then Actual = $2,331.42

Effective = $2,331.42

## C. Variable bill estimate

Historical fully paid amounts:

- 173.81
- 162.44
- 176.22
- 168.90
- 159.33
- 161.77

Expected estimate:

Arithmetic mean rounded to cents.

If occurrence-specific estimate is entered, it overrides the mean.

## D. Protected buffer is not an expense

Starting balance: $1,000  
Buffer: $500  
No bills  
No paychecks

Expected safe-to-spend:

$500

The system must not create a `$500` outgoing transaction.

With three future paychecks, it must still not subtract `$500` three times.

## E. Basic latest-paycheck assignment

Paychecks:

- Aug 14: $2,000
- Aug 21: $1,500

Bill:

- due Aug 18
- $200
- manual
- no special window

Expected funding:

Aug 14 paycheck.

## F. Latest eligible paycheck

Paychecks:

- Aug 1
- Aug 14
- Aug 28

Bill due Aug 25.

Expected default funding:

Aug 14, not Aug 1.

## G. Move earlier to protect buffer

Starting/carry assumptions should create:

- Aug 21 assignment would project below $500 buffer
- Aug 14 has enough spare headroom

Expected:

Scheduler moves an eligible manual bill to Aug 14.

Decision reason:

`MOVED_EARLIER_TO_PROTECT_BUFFER`

No bill becomes late.

## H. Autopay cannot be casually moved

Autopay:

- draft Aug 23
- $24

Expected:

Draft date remains Aug 23.

Scheduler may change which prior paycheck funds it, but not the actual autopay date.

## I. Locked bill

User locks Mortgage to Aug 14 funding.

Recalculate.

Expected:

Lock preserved.

If lock would make Mortgage late:

Expected:

Warning requiring user action.

Do not silently move.

## J. Partial payment

Bill amount: $600

Payment 1: $300

Expected:

Status = Partial  
Remaining = $300

Payment 2: $300

Expected:

Status = Paid  
Remaining = $0

Ordinary Mark Paid control no longer available.

## K. Split funding

Bill: $600 due Aug 28

Allocations:

- Aug 14: $300
- Aug 21: $300

Expected:

Funded amount = $600  
Funding status = fully funded

Paid status remains Unpaid until actual payment is entered.

## L. Higher paycheck

Expected paycheck: $2,000  
Actual: $2,400

Expected:

- plan recalculates
- $400 incremental surplus is not automatically assigned to spending/savings
- UI surfaces surplus as available/uncommitted

## M. Lower paycheck

Expected paycheck: $2,000  
Actual: $1,600

Required bills still must be paid.

Optional savings: $200  
Extra debt: $100

Expected shortage-resolution order:

- reduce extra debt first
- reduce optional savings next
- rebalance manual bills if eligible
- safe-to-spend reduced
- unresolved shortage shown if still insufficient

## N. Essential priority

Available funding insufficient for:

- Mortgage $1,100 Essential
- Netflix $24 Flexible
- Optional Savings $200

Expected:

Mortgage protected.

Optional savings reduced before jeopardizing Mortgage.

Netflix scheduling still respects due date; if truly impossible, shortage is shown rather than pretending it is paid.

## O. Balance reconciliation

App balance: $3,902.91  
Actual entered: $3,847.52

Difference:

`-$55.39`

If user selects Untracked Spending:

- record reconciliation
- create appropriate adjustment/spending record
- new app balance = $3,847.52
- schedule recalculates

## P. Duplicate payment protection

Bill already fully paid.

Expected:

Normal `Mark Paid` action unavailable.

Editing existing payment remains possible.

Additional payment requires explicit advanced action.

## Q. Weekend due date

Due date falls Sunday.

No provider-specific override.

Expected latest payment date:

prior business day.

Per-bill user override can change the behavior.

## R. Schedule stability

Existing plan is valid and buffer-safe.

Small unrelated note edit occurs.

Expected:

Scheduler should not arbitrarily move bills.

## S. Manual move simulation

Moving Electric from Paycheck A to Paycheck B would cause buffer shortfall of $23.

Before save, UI must show:

- before/after safe-to-spend or headroom
- buffer warning
- destination impact

User can cancel.

If user confirms an allowed override, save and recalculate.

## T. Paid-by visibility

Jonathan marks Electric paid.

Switch to Tiffany profile.

Expected:

Electric visibly says Paid and identifies Jonathan/date/amount.

No duplicate payment prompt.

## U. Occurrence override

Recurring bill normally due 18th.

September occurrence changed to Sept 20 only.

Expected:

- September uses Sept 20
- October returns to normal recurrence
- September override preserved in history

## V. Archive bill

Netflix archived.

Expected:

- future occurrences no longer generated after archive effective point
- historical payments remain accessible

## W. Backup and restore

1. Create data.
2. Create backup.
3. Change/delete/archive records.
4. Restore backup.

Expected:

- restored data matches backup
- app starts successfully
- schedule recalculates
- backup integrity verified

## X. Migration

Start from prior schema fixture.

Run new version.

Expected:

- automatic pre-migration backup
- migration succeeds
- existing bills/payments remain intact

## Y. AI disabled

AI not installed.

Expected:

Every non-AI feature works.

No screen blocks on AI.

## Z. AI factual grounding

Deterministic engine says:

- buffer = $500
- headroom = $461.46
- status = Tight

Ask AI why paycheck is tight.

Expected:

AI explanation uses these supplied amounts.

It must not fabricate a different balance.

## AA. AI proposal cannot mutate directly

Ask AI:

`Move Electric to the prior paycheck.`

Expected:

- AI returns proposal
- deterministic simulation runs
- review card shown
- DB unchanged until user clicks Apply
- after Apply, normal Rust command validates and commits

## AB. Malformed AI output

Model returns invalid proposal JSON.

Expected:

- no mutation
- safe error/retry
- financial app remains usable

## AC. Clean install

On clean Windows 11 Home:

- run `HouseholdBillsSetup.exe`
- install
- launch

Expected:

No manual installation of Node, Rust, Python, Docker, PostgreSQL, or SQLite CLI.

## AD. Offline core use

Disconnect internet.

Expected:

Bills, paychecks, scheduler, reconciliation, backups, and reports continue working.

Local AI works if model/runtime already installed.
