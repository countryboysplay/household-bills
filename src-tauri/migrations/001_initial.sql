CREATE TABLE IF NOT EXISTS app_meta (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  onboarding_complete INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
INSERT OR IGNORE INTO app_meta(id,onboarding_complete) VALUES (1,0);

CREATE TABLE IF NOT EXISTS users (
  id TEXT PRIMARY KEY,
  display_name TEXT NOT NULL,
  pin_hash TEXT,
  avatar_key TEXT,
  theme_preference TEXT NOT NULL DEFAULT 'system' CHECK(theme_preference IN ('system','light','dark')),
  is_active INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS accounts (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  account_type TEXT NOT NULL CHECK(account_type IN ('checking','savings','cash','credit','other')),
  book_balance_cents INTEGER NOT NULL DEFAULT 0,
  is_primary_bill_account INTEGER NOT NULL DEFAULT 0,
  is_active INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  archived_at TEXT
);

CREATE TABLE IF NOT EXISTS household_settings (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  household_name TEXT NOT NULL,
  primary_account_id TEXT REFERENCES accounts(id),
  protected_buffer_cents INTEGER NOT NULL DEFAULT 50000,
  tight_headroom_cents INTEGER NOT NULL DEFAULT 10000,
  default_planning_horizon_days INTEGER NOT NULL DEFAULT 90,
  ai_enabled INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS categories (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  kind TEXT NOT NULL,
  icon_key TEXT,
  is_system INTEGER NOT NULL DEFAULT 0,
  is_active INTEGER NOT NULL DEFAULT 1,
  sort_order INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS income_sources (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL REFERENCES users(id),
  name TEXT NOT NULL,
  schedule_type TEXT NOT NULL,
  schedule_config_json TEXT NOT NULL DEFAULT '{}',
  default_projected_amount_cents INTEGER NOT NULL DEFAULT 0,
  weekend_holiday_rule TEXT NOT NULL DEFAULT 'prior_business_day',
  is_active INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  archived_at TEXT
);

CREATE TABLE IF NOT EXISTS paycheck_occurrences (
  id TEXT PRIMARY KEY,
  income_source_id TEXT NOT NULL REFERENCES income_sources(id),
  pay_date TEXT NOT NULL,
  projected_amount_cents INTEGER NOT NULL,
  expected_amount_cents INTEGER,
  actual_amount_cents INTEGER,
  status TEXT NOT NULL DEFAULT 'projected' CHECK(status IN ('projected','updated','received','skipped')),
  is_date_override INTEGER NOT NULL DEFAULT 0,
  note TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_paychecks_date ON paycheck_occurrences(pay_date);

CREATE TABLE IF NOT EXISTS bill_templates (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  category_id TEXT REFERENCES categories(id),
  amount_type TEXT NOT NULL CHECK(amount_type IN ('fixed','variable')),
  fixed_amount_cents INTEGER,
  fallback_estimate_cents INTEGER,
  estimate_window_count INTEGER NOT NULL DEFAULT 6,
  recurrence_type TEXT NOT NULL DEFAULT 'monthly',
  recurrence_config_json TEXT NOT NULL DEFAULT '{}',
  due_rule_json TEXT NOT NULL DEFAULT '{}',
  payment_type TEXT NOT NULL CHECK(payment_type IN ('manual','autopay')),
  priority TEXT NOT NULL DEFAULT 'normal' CHECK(priority IN ('essential','normal','flexible')),
  payment_window_type TEXT NOT NULL DEFAULT 'anytime',
  payment_window_config_json TEXT NOT NULL DEFAULT '{}',
  can_split INTEGER NOT NULL DEFAULT 0,
  assigned_user_id TEXT REFERENCES users(id),
  preferred_paycheck_rule_json TEXT,
  is_active INTEGER NOT NULL DEFAULT 1,
  notes TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  archived_at TEXT
);

CREATE TABLE IF NOT EXISTS bill_occurrences (
  id TEXT PRIMARY KEY,
  bill_template_id TEXT REFERENCES bill_templates(id),
  name_snapshot TEXT NOT NULL,
  category_id TEXT REFERENCES categories(id),
  due_date TEXT NOT NULL,
  latest_payment_date TEXT NOT NULL,
  earliest_payment_date TEXT NOT NULL,
  estimated_amount_cents INTEGER NOT NULL,
  manual_amount_override_cents INTEGER,
  actual_required_amount_cents INTEGER,
  status TEXT NOT NULL DEFAULT 'upcoming' CHECK(status IN ('upcoming','scheduled','partial','paid','late','skipped')),
  payment_type_snapshot TEXT NOT NULL,
  priority_snapshot TEXT NOT NULL,
  is_one_time INTEGER NOT NULL DEFAULT 0,
  is_due_date_override INTEGER NOT NULL DEFAULT 0,
  scheduled_payment_date TEXT,
  lock_type TEXT NOT NULL DEFAULT 'none',
  lock_metadata_json TEXT,
  notes TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_bill_occurrences_due ON bill_occurrences(due_date,status);

CREATE TABLE IF NOT EXISTS bill_allocations (
  id TEXT PRIMARY KEY,
  bill_occurrence_id TEXT NOT NULL REFERENCES bill_occurrences(id) ON DELETE CASCADE,
  paycheck_occurrence_id TEXT REFERENCES paycheck_occurrences(id),
  funding_source_type TEXT NOT NULL CHECK(funding_source_type IN ('current_cash','paycheck')),
  allocated_amount_cents INTEGER NOT NULL CHECK(allocated_amount_cents >= 0),
  source TEXT NOT NULL CHECK(source IN ('scheduler','manual')),
  is_locked INTEGER NOT NULL DEFAULT 0,
  reason_code TEXT,
  created_by_user_id TEXT REFERENCES users(id),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS payments (
  id TEXT PRIMARY KEY,
  bill_occurrence_id TEXT NOT NULL REFERENCES bill_occurrences(id),
  account_id TEXT REFERENCES accounts(id),
  amount_cents INTEGER NOT NULL CHECK(amount_cents > 0),
  paid_date TEXT NOT NULL,
  paid_by_user_id TEXT NOT NULL REFERENCES users(id),
  payment_method TEXT,
  note TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS transactions (
  id TEXT PRIMARY KEY,
  account_id TEXT NOT NULL REFERENCES accounts(id),
  transaction_date TEXT NOT NULL,
  description TEXT NOT NULL,
  category_id TEXT REFERENCES categories(id),
  amount_cents INTEGER NOT NULL,
  transaction_type TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'cleared',
  source TEXT NOT NULL DEFAULT 'manual',
  created_by_user_id TEXT REFERENCES users(id),
  note TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS balance_reconciliations (
  id TEXT PRIMARY KEY,
  account_id TEXT NOT NULL REFERENCES accounts(id),
  app_balance_before_cents INTEGER NOT NULL,
  actual_balance_cents INTEGER NOT NULL,
  difference_cents INTEGER NOT NULL,
  resolution_type TEXT NOT NULL,
  note TEXT,
  performed_by_user_id TEXT REFERENCES users(id),
  performed_at TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS savings_goals (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  goal_type TEXT NOT NULL,
  target_amount_cents INTEGER NOT NULL DEFAULT 0,
  target_date TEXT,
  current_amount_cents INTEGER NOT NULL DEFAULT 0,
  planned_contribution_cents INTEGER NOT NULL DEFAULT 0,
  contribution_frequency TEXT,
  is_required_contribution INTEGER NOT NULL DEFAULT 0,
  is_active INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  archived_at TEXT
);

CREATE TABLE IF NOT EXISTS debts (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  balance_cents INTEGER NOT NULL DEFAULT 0,
  apr_basis_points INTEGER NOT NULL DEFAULT 0,
  minimum_payment_cents INTEGER NOT NULL DEFAULT 0,
  planned_payment_cents INTEGER NOT NULL DEFAULT 0,
  is_active INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  archived_at TEXT
);

CREATE TABLE IF NOT EXISTS activity_log (
  id TEXT PRIMARY KEY,
  user_id TEXT REFERENCES users(id),
  event_type TEXT NOT NULL,
  entity_type TEXT,
  entity_id TEXT,
  summary TEXT NOT NULL,
  metadata_json TEXT,
  occurred_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
