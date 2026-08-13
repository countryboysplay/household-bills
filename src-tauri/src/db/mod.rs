use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use crate::{domain::money::Money, error::AppResult};

const MIGRATIONS: &[(&str, &str)] = &[
    ("001_initial", include_str!("../../migrations/001_initial.sql")),
    ("002_phase3", include_str!("../../migrations/002_phase3.sql")),
    ("003_paycheck_schedules", include_str!("../../migrations/003_paycheck_schedules.sql")),
    ("004_due_date_semantics", include_str!("../../migrations/004_due_date_semantics.sql")),
    ("005_phase5", include_str!("../../migrations/005_phase5.sql")),
    ("006_release_1_0", include_str!("../../migrations/006_release_1_0.sql")),
];

pub fn has_pending_migrations(conn: &Connection) -> AppResult<bool> {
    let migration_table_exists: i64 = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='schema_migrations')",
        [],
        |r| r.get(0),
    )?;
    if migration_table_exists == 0 { return Ok(true); }
    for &(version, _) in MIGRATIONS {
        let applied: Option<String> = conn.query_row(
            "SELECT version FROM schema_migrations WHERE version=?1",
            [version],
            |r| r.get(0),
        ).optional()?;
        if applied.is_none() { return Ok(true); }
    }
    Ok(false)
}

pub fn initialize(conn: &mut Connection) -> AppResult<()> {
    conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;")?;
    conn.execute_batch("CREATE TABLE IF NOT EXISTS schema_migrations (version TEXT PRIMARY KEY, applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);")?;
    for &(version, sql) in MIGRATIONS {
        let applied: Option<String> = conn.query_row("SELECT version FROM schema_migrations WHERE version=?1", [version], |r| r.get(0)).optional()?;
        if applied.is_none() {
            let tx = conn.transaction()?;
            tx.execute_batch(sql)?;
            tx.execute("INSERT INTO schema_migrations(version) VALUES (?1)", [version])?;
            tx.commit()?;
        }
    }
    seed_categories(conn)?;
    Ok(())
}

fn seed_categories(conn: &Connection) -> AppResult<()> {
    let defaults = [
        ("housing","Housing","bill","home",10),("utilities","Utilities","bill","bolt",20),("insurance","Insurance","bill","shield",30),("debt","Debt","debt","card",40),
        ("groceries","Groceries","spending","cart",50),("gas","Gas","spending","fuel",60),("dining","Dining Out","spending","food",70),("shopping","Shopping","spending","bag",80),
        ("household","Household","spending","home",90),("income","Income","income","money",100),("savings","Savings","savings","piggy",110),("other","Other","other","dots",999),
    ];
    for (id,name,kind,icon,sort) in defaults {
        conn.execute("INSERT OR IGNORE INTO categories(id,name,kind,icon_key,is_system,is_active,sort_order) VALUES (?1,?2,?3,?4,1,1,?5)", params![id,name,kind,icon,sort])?;
    }
    Ok(())
}

pub fn onboarding_complete(conn: &Connection) -> AppResult<bool> {
    Ok(conn.query_row("SELECT onboarding_complete FROM app_meta WHERE id=1", [], |r| r.get::<_, i64>(0)).optional()?.unwrap_or(0) == 1)
}

pub fn complete_onboarding(conn: &mut Connection, household_name: &str, buffer: Money, account_name: &str, account_balance: Money, users: &[String]) -> AppResult<()> {
    let tx = conn.transaction()?;
    let account_id = Uuid::new_v4().to_string();
    tx.execute("INSERT INTO accounts(id,name,account_type,book_balance_cents,is_primary_bill_account,is_active) VALUES (?1,?2,'checking',?3,1,1)", params![account_id,account_name,account_balance.value()])?;
    tx.execute("INSERT OR REPLACE INTO household_settings(id,household_name,primary_account_id,protected_buffer_cents,tight_headroom_cents,default_planning_horizon_days,ai_enabled,updated_at) VALUES (1,?1,?2,?3,10000,90,0,CURRENT_TIMESTAMP)", params![household_name,account_id,buffer.value()])?;
    for name in users {
        if !name.trim().is_empty() {
            tx.execute("INSERT INTO users(id,display_name,theme_preference,is_active) VALUES (?1,?2,'system',1)", params![Uuid::new_v4().to_string(),name.trim()])?;
        }
    }
    tx.execute("UPDATE app_meta SET onboarding_complete=1, updated_at=CURRENT_TIMESTAMP WHERE id=1", [])?;
    tx.commit()?;
    Ok(())
}
