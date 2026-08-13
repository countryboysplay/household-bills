# Hotfix 3.5

- Keep contractual bill due dates separate from prior-business-day planning deadlines.
- Regenerate future unpaid monthly occurrences using the corrected due-date semantics.
- Separate paycheck funding/reservation from actual split payments. A normal one-payment bill may reserve funds across multiple earlier paychecks without being partially paid.
- Planner labels multi-paycheck funding as Reserve for <bill>.
- Partial-payment action is only shown when the bill allows actual split payments.
- Add friendlier funding warnings instead of raw bill UUIDs.
- Add regression tests for weekend due dates and multi-paycheck reservation of a non-splittable bill.
