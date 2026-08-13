ALTER TABLE household_settings ADD COLUMN backup_retention_count INTEGER NOT NULL DEFAULT 14;
ALTER TABLE savings_goals ADD COLUMN notes TEXT;
ALTER TABLE debts ADD COLUMN due_day INTEGER;
ALTER TABLE bill_allocations ADD COLUMN recommended_payment_date TEXT;

CREATE TABLE IF NOT EXISTS savings_contributions (
  id TEXT PRIMARY KEY,
  goal_id TEXT NOT NULL REFERENCES savings_goals(id) ON DELETE CASCADE,
  account_id TEXT REFERENCES accounts(id),
  amount_cents INTEGER NOT NULL CHECK(amount_cents > 0),
  contribution_date TEXT NOT NULL,
  contributed_by_user_id TEXT REFERENCES users(id),
  note TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_savings_contrib_goal ON savings_contributions(goal_id, contribution_date);

CREATE TABLE IF NOT EXISTS debt_payments (
  id TEXT PRIMARY KEY,
  debt_id TEXT NOT NULL REFERENCES debts(id) ON DELETE CASCADE,
  account_id TEXT REFERENCES accounts(id),
  amount_cents INTEGER NOT NULL CHECK(amount_cents > 0),
  payment_date TEXT NOT NULL,
  paid_by_user_id TEXT REFERENCES users(id),
  is_extra INTEGER NOT NULL DEFAULT 1,
  note TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_debt_payments_debt ON debt_payments(debt_id, payment_date);
