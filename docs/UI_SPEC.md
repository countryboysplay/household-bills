# UI Specification

> **Phase 5 scope override (current):** AI has been removed from the Household Bills roadmap. Any AI, llama.cpp, model, Ask AI, or AI proposal material later in this historical document is superseded by `AI_SPEC.md` and must not be implemented. The product remains deterministic and local.

## 1. Design authority

The approved screenshots in `design_references/` define the desktop visual direction.

Primary style reference:

`01_dashboard.png`

Use the other images for screen-specific layout.

The screenshots are generated mockups. Their sample data is not authoritative. Do not reproduce numerical inconsistencies.

## 2. Visual language

### Overall

- Windows desktop application
- dark navy left sidebar
- light main content surface
- rounded cards
- subtle borders
- restrained shadows
- blue as primary action color
- green for healthy/paid states
- amber/orange for caution
- red only for actual problems
- purple accent for local AI
- generous spacing without wasting desktop real estate
- readable financial typography

### Typography

Use a clean Windows-friendly sans-serif stack.

Financial totals should be easy to scan.

Avoid tiny text except metadata.

### Interaction

- desktop-first
- mouse/keyboard friendly
- clear hover/focus states
- no mobile-specific bottom navigation
- no swipe-only interactions
- sidebar collapsible
- tables sortable where useful
- dialogs/drawers for focused editing

## 3. Sidebar

Primary navigation:

1. Dashboard
2. Paycheck Planner
3. Bills
4. Calendar
5. Spending
6. Savings & Debt
7. History
8. Ask AI
9. Settings

Bottom area may show:

- active profile
- database/local status
- backup status
- AI model/status

Keep system status compact.

## 4. Dashboard

Reference: `01_dashboard.png`

### Purpose

Answer household status immediately.

### Top summary cards

Show:

- Current/Total Cash
- Next Paycheck
- Safe to Spend
- Monthly Progress or Reserved/Buffer status

Do not mislabel total balances.

### Main grid

Sections:

- Upcoming Bills
- Cash Flow
- Paycheck Overview
- Spending by Category
- Savings & Debt Overview
- Recent Activity

Right rail:

- Ask Household AI
- Alerts
- Quick Actions

### Priority behavior

If there is a real shortage, surface it prominently.

Do not show multiple giant warning panels.

### AI

AI card is secondary.

It may show a deterministic insight and an explanation generated locally.

Any suggested change opens a normal review flow.

## 5. Paycheck Planner

Reference: `02_paycheck_planner.png`

### Purpose

This is the core working screen.

Show upcoming paycheck buckets/cards horizontally or in a dense desktop grid.

Each paycheck card should show:

- date
- owner
- effective paycheck amount
- assigned obligations
- protected buffer status
- safe-to-spend/headroom
- Healthy/Tight/Shortage state

### Important correction to mockup

Do not display "Auto-Plan AI."

Scheduling is deterministic.

Use labels such as:

- Auto-plan: On
- Recalculate Schedule
- Scheduler Rules

AI may explain the resulting plan but does not create the authoritative schedule.

### Controls

- horizon selector
- Add Paycheck
- Recalculate Schedule
- Paycheck Settings
- Print optional
- Ask Household AI optional

### Below cards

Show:

- Unscheduled/undecided items
- Schedule alerts
- concise scheduler notes
- current protected buffer setting

## 6. Bills screen

Reference: `03_bills.png`

### Main list

Columns:

- Bill
- Amount
- Due
- Frequency
- Type
- Status
- Assigned From
- actions

Filters:

- All
- Due Soon
- Autopay
- Unpaid
- Paid
- Archived

Top actions:

- Add Bill
- optional Import Bills
- Ask AI

### Detail drawer

Selecting a bill opens right-side detail panel.

Show:

- next due date
- amount
- frequency
- payment type
- priority
- assigned/responsible user
- scheduled funding paycheck
- payment history
- year-to-date summary

Actions:

- Mark Paid
- Edit
- Move Paycheck
- More

## 7. Add/Edit Bill

Reference: `04_add_edit_bill.png`

### Basic Information

Required:

- Bill Name
- Category
- Fixed/Variable
- Amount/Estimate
- Due Date
- Recurrence
- Manual/Autopay

### More Options

Keep collapsed or visually secondary.

Include:

- Priority
- Earliest Payment / payment window
- Can Split
- Assigned To
- Preferred Paycheck Rule
- Notes

### Remove from mockup

Do not implement complex Tags in V1.

### Preview

Right-side preview can show current interpretation and recent history for edits.

## 8. Bill Detail

Reference: `05_bill_detail.png`

Main area:

- summary header
- amount trend
- payment history
- notes
- optional attachments

Right rail:

- Mark as Paid form
- upcoming occurrences
- Ask AI about this bill

Mark Paid form:

- expected amount
- actual amount
- paid date
- paid by
- optional payment method
- optional note
- Partial Payment

AI summary must use deterministic history values.

## 9. Spending & Reconciliation

Reference: `06_spending_reconciliation.png`

### Top accounts

Cards for manually tracked accounts.

Primary account should be obvious.

### Main transaction table

Filters:

- date range
- account
- category
- status
- search

Quick add buttons can support:

- Groceries
- Gas
- Dining Out
- Shopping
- Income
- Other

### Reconciliation panel

Prominent.

Show:

- app balance
- user-entered actual balance
- difference
- last reconciled
- Reconcile Now

Do not use bank-specific words such as "Statement Balance" unless the user manually enters a statement balance.

### Spending summaries

Keep simple:

- month spending
- category budget progress
- total income/spending/net cash flow

## 10. Calendar

Reference: `07_calendar.png`

Primary month view.

Display:

- paychecks
- bill due events
- manual recommended pay dates
- autopays
- one-time items

Right-side day detail:

- date
- paycheck if applicable
- bills
- amounts
- safe-to-spend/paycheck summary

Bottom may show upcoming paycheck overview.

## 11. Savings & Debt

Reference: `08_savings_debt.png`

### Top summary

- total saved
- planned contributions
- total debt
- planned extra payment
- net monthly progress

### Main sections

Tabs/sections:

- Savings Goals
- Sinking Funds
- Debt Payoff

Show simple progress bars.

Debt comparison:

- Snowball
- Avalanche
- Custom

Recommendation must come from deterministic calculations.

AI may explain tradeoffs.

### Scope restraint

Do not add investment tracking.

## 12. History

No approved image exists, so use dashboard style.

Recommended layout:

Top:

- History
- date range
- type filter
- profile filter
- search

Main table/timeline:

- timestamp/date
- action
- object
- amount if applicable
- performed by
- details

Types:

- Payment
- Paycheck Update
- Bill Change
- Transaction
- Reconciliation
- Scheduler Override
- Backup/Restore optional

Right detail drawer shows metadata when selected.

Keep this practical, not forensic.

## 13. Ask AI screen

Use dashboard visual language.

Layout:

Left/main chat area.

Right context rail:

- AI status
- model
- loaded context type
- privacy statement: Local only
- recent suggested questions

Example prompts:

- What bills still need to be paid this week?
- Why is the next paycheck tight?
- If Jonathan's next check is $400 lower, what changes?
- How much did we spend on utilities this year?
- Can we safely make an extra $200 debt payment?

When AI proposes a change, render a structured proposal card with:

- proposed action
- before
- after
- deterministic impact
- Apply Change
- Cancel

Apply Change calls normal validated application command.

## 14. Settings

Use simple grouped cards.

### Household

- Household name
- Jonathan profile
- Tiffany profile
- active/default profile

### Financial

- Primary bill-paying account
- Protected buffer
- Tight threshold
- Default planning horizon
- optional spending budgets

### Scheduler

- Auto-recalculate toggle
- weekend/holiday defaults
- optional savings reduction behavior

### AI

- Enable/disable
- model status
- model path/download
- start/stop test
- context size/performance preset if needed
- local-only status

Avoid exposing dozens of llama.cpp flags.

### Backup

- backup folder
- automatic backups on/off
- retention count
- Create Backup
- Restore Backup
- Open Backup Folder

### Appearance

- System
- Light
- Dark

### About

- version
- database schema version
- app data folder
- logs folder
- check for update if updater exists

## 15. First-run onboarding

Keep it wizard-like and short.

1. Welcome
2. Profiles
3. Primary account + balance
4. Protected buffer
5. Paychecks
6. Bills
7. Review first 90-day plan
8. Done

AI setup should be optional after core financial setup.

## 16. Dialog standards

Destructive actions need confirmation.

Financial changes that affect schedule should show impact when material.

Do not require confirmations for harmless navigation.

## 17. Empty states

Every major screen needs useful empty states.

Examples:

Bills:
`No bills yet. Add your first recurring bill to start building the paycheck plan.`

Paychecks:
`Add a paycheck schedule so Household Bills can begin assigning upcoming bills.`

AI disabled:
`Local AI is off. The financial planner continues to work normally.`

## 18. Accessibility

- keyboard focus
- accessible names
- do not rely on color alone
- sufficient contrast
- status icon + text
- reasonable table row height
- scalable text

## 19. What not to copy from mockups

Generated screenshots contain illustrative labels that may conflict with the product rules.

Do not implement:

- bank synchronization claims
- automatic imported bank transactions
- "Auto-Plan AI"
- buffer as repeated per-paycheck expense
- values that do not reconcile mathematically
- mobile navigation
- complex tag system

Use the screenshots for layout and visual hierarchy only where behavior differs.
