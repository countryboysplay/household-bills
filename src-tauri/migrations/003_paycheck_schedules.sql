ALTER TABLE paycheck_occurrences ADD COLUMN scheduled_pay_date TEXT;
UPDATE paycheck_occurrences SET scheduled_pay_date=pay_date WHERE scheduled_pay_date IS NULL AND is_date_override=0;
CREATE INDEX IF NOT EXISTS idx_paychecks_scheduled_date ON paycheck_occurrences(scheduled_pay_date);
