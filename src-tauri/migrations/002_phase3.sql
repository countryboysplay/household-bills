ALTER TABLE bill_templates ADD COLUMN due_day INTEGER;
ALTER TABLE bill_templates ADD COLUMN pay_earliest_days_before INTEGER NOT NULL DEFAULT 31;
ALTER TABLE paycheck_occurrences ADD COLUMN posted_to_account INTEGER NOT NULL DEFAULT 0;
ALTER TABLE transactions ADD COLUMN source_entity_type TEXT;
ALTER TABLE transactions ADD COLUMN source_entity_id TEXT;

CREATE INDEX IF NOT EXISTS idx_bill_template_active ON bill_templates(is_active, archived_at);
CREATE INDEX IF NOT EXISTS idx_bill_allocations_bill ON bill_allocations(bill_occurrence_id);
CREATE INDEX IF NOT EXISTS idx_bill_allocations_paycheck ON bill_allocations(paycheck_occurrence_id);
CREATE INDEX IF NOT EXISTS idx_payments_bill ON payments(bill_occurrence_id, paid_date);
CREATE INDEX IF NOT EXISTS idx_transactions_entity ON transactions(source_entity_type, source_entity_id);
CREATE UNIQUE INDEX IF NOT EXISTS ux_bill_occurrence_template_due
  ON bill_occurrences(bill_template_id, due_date)
  WHERE bill_template_id IS NOT NULL;
