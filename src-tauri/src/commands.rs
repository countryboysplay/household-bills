use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::{backup, db, domain::money::Money, error::{AppError, AppResult}, AppState};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserProfileDto { pub id: String, pub display_name: String }

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountDto { pub id: String, pub name: String, pub account_type: String, pub book_balance_cents: i64, pub is_primary_bill_account: bool }

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsDto { pub household_name: String, pub protected_buffer_cents: i64, pub default_planning_horizon_days: i64, pub ai_enabled: bool }

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapDto { pub app_version: String, pub onboarding_complete: bool, pub users: Vec<UserProfileDto>, pub accounts: Vec<AccountDto>, pub settings: Option<SettingsDto>, pub database_path: String, pub backup_directory: String, pub restore_error: Option<String> }

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingPayload { pub household_name: String, pub protected_buffer_cents: i64, pub primary_account_name: String, pub primary_account_balance_cents: i64, pub users: Vec<String> }

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSummaryDto { pub current_cash_cents: i64, pub safe_to_spend_cents: i64, pub reserved_bills_cents: i64, pub protected_buffer_cents: i64, pub upcoming_bill_count: i64, pub next_paycheck_date: Option<String>, pub next_paycheck_owner: Option<String>, pub next_paycheck_amount_cents: Option<i64> }

fn bootstrap_from_state(state: &AppState) -> AppResult<BootstrapDto> {
    let conn = state.db.lock().map_err(|_| AppError::Validation("database lock poisoned".into()))?;
    let onboarding_complete = db::onboarding_complete(&conn)?;
    let mut users_stmt = conn.prepare("SELECT id,display_name FROM users WHERE is_active=1 ORDER BY created_at")?;
    let users = users_stmt.query_map([], |r| Ok(UserProfileDto { id:r.get(0)?, display_name:r.get(1)? }))?.collect::<Result<Vec<_>,_>>()?;
    let mut accounts_stmt = conn.prepare("SELECT id,name,account_type,book_balance_cents,is_primary_bill_account FROM accounts WHERE is_active=1 ORDER BY is_primary_bill_account DESC, created_at")?;
    let accounts = accounts_stmt.query_map([], |r| Ok(AccountDto { id:r.get(0)?, name:r.get(1)?, account_type:r.get(2)?, book_balance_cents:r.get(3)?, is_primary_bill_account:r.get::<_,i64>(4)?==1 }))?.collect::<Result<Vec<_>,_>>()?;
    let settings = conn.query_row("SELECT household_name,protected_buffer_cents,default_planning_horizon_days,ai_enabled FROM household_settings WHERE id=1", [], |r| Ok(SettingsDto { household_name:r.get(0)?, protected_buffer_cents:r.get(1)?, default_planning_horizon_days:r.get(2)?, ai_enabled:r.get::<_,i64>(3)?==1 })).optional()?;
    Ok(BootstrapDto { app_version: env!("CARGO_PKG_VERSION").into(), onboarding_complete, users, accounts, settings, database_path:state.database_path.display().to_string(), backup_directory:state.backup_dir.display().to_string(), restore_error:state.restore_error.clone() })
}

#[tauri::command]
pub fn get_app_bootstrap(state: State<'_, AppState>) -> AppResult<BootstrapDto> { bootstrap_from_state(&state) }

#[tauri::command]
pub fn complete_onboarding(payload: OnboardingPayload, state: State<'_, AppState>) -> AppResult<BootstrapDto> {
    if payload.protected_buffer_cents < 0 { return Err(AppError::Validation("protected buffer cannot be negative".into())); }
    if payload.users.is_empty() { return Err(AppError::Validation("at least one household profile is required".into())); }
    let mut conn = state.db.lock().map_err(|_| AppError::Validation("database lock poisoned".into()))?;
    if db::onboarding_complete(&conn)? { return Err(AppError::Validation("household onboarding is already complete".into())); }
    db::complete_onboarding(&mut conn, payload.household_name.trim(), Money::cents(payload.protected_buffer_cents), payload.primary_account_name.trim(), Money::cents(payload.primary_account_balance_cents), &payload.users)?;
    drop(conn);
    bootstrap_from_state(&state)
}

#[tauri::command]
pub fn get_dashboard_summary(state: State<'_, AppState>) -> AppResult<DashboardSummaryDto> {
    let conn = state.db.lock().map_err(|_| AppError::Validation("database lock poisoned".into()))?;
    let current_cash: i64 = conn.query_row("SELECT COALESCE(SUM(book_balance_cents),0) FROM accounts WHERE is_active=1 AND account_type != 'credit'", [], |r| r.get(0))?;
    let buffer: i64 = conn.query_row("SELECT COALESCE(protected_buffer_cents,0) FROM household_settings WHERE id=1", [], |r| r.get(0)).optional()?.unwrap_or(0);
    // Only current-cash allocations reduce money that is safe to spend today.
    // Allocations assigned to future paychecks are funded by those future deposits and
    // must not be subtracted from today's account balance.
    let reserved: i64 = conn.query_row("SELECT COALESCE(SUM(allocated_amount_cents),0) FROM bill_allocations a JOIN bill_occurrences b ON b.id=a.bill_occurrence_id WHERE b.status IN ('upcoming','scheduled','partial') AND a.funding_source_type='current_cash'", [], |r| r.get(0))?;
    let safe = (current_cash - reserved - buffer).max(0);
    let upcoming_bill_count: i64 = conn.query_row("SELECT COUNT(*) FROM bill_occurrences WHERE status IN ('upcoming','scheduled','partial')", [], |r| r.get(0))?;
    let next = conn.query_row("SELECT p.pay_date, u.display_name, COALESCE(p.actual_amount_cents,p.expected_amount_cents,p.projected_amount_cents) FROM paycheck_occurrences p JOIN income_sources i ON i.id=p.income_source_id JOIN users u ON u.id=i.user_id WHERE p.status IN ('projected','updated') AND p.pay_date >= date('now','localtime') ORDER BY p.pay_date LIMIT 1", [], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,i64>(2)?))).optional()?;
    let (next_paycheck_date,next_paycheck_owner,next_paycheck_amount_cents) = match next { Some((d,o,a)) => (Some(d),Some(o),Some(a)), None => (None,None,None) };
    Ok(DashboardSummaryDto { current_cash_cents:current_cash, safe_to_spend_cents:safe, reserved_bills_cents:reserved, protected_buffer_cents:buffer, upcoming_bill_count, next_paycheck_date, next_paycheck_owner, next_paycheck_amount_cents })
}

#[tauri::command]
pub fn create_backup(state: State<'_, AppState>) -> AppResult<String> {
    let conn = state.db.lock().map_err(|_| AppError::Validation("database lock poisoned".into()))?;
    let retention: i64 = conn.query_row(
        "SELECT COALESCE(backup_retention_count,14) FROM household_settings WHERE id=1",
        [],
        |r| r.get(0),
    ).unwrap_or(14);
    let path = backup::create_backup(&conn, &state.database_path, &state.backup_dir)?;
    drop(conn);
    backup::prune_backups(&state.backup_dir, retention.max(3) as usize)?;
    Ok(path.display().to_string())
}
