mod backup;
mod commands;
mod db;
pub mod domain;
mod error;
mod phase3;
mod phase4;
mod phase5;
mod updates;
pub mod scheduler;

use std::{fs, path::PathBuf, sync::Mutex};
use rusqlite::Connection;
use tauri::Manager;

pub struct AppState {
    db: Mutex<Connection>,
    database_path: PathBuf,
    backup_dir: PathBuf,
    export_dir: PathBuf,
    restore_marker: PathBuf,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir().map_err(|e| std::io::Error::other(format!("app data directory unavailable: {e}")))?;
            fs::create_dir_all(&app_data_dir)?;
            let backup_dir = app_data_dir.join("Backups");
            let export_dir = app_data_dir.join("Exports");
            fs::create_dir_all(&backup_dir)?;
            fs::create_dir_all(&export_dir)?;
            let database_path = app_data_dir.join("household_bills.sqlite3");
            let restore_marker = app_data_dir.join("restore_pending.txt");
            backup::apply_pending_restore(&database_path, &restore_marker)?;
            let database_existed = database_path.is_file();
            let mut conn = Connection::open(&database_path)?;
            let pending_migrations = database_existed && db::has_pending_migrations(&conn).unwrap_or(true);
            // Protect existing household data before every schema upgrade.
            if pending_migrations {
                backup::create_backup(&conn, &database_path, &backup_dir)?;
            }
            db::initialize(&mut conn)?;
            let current_version = env!("CARGO_PKG_VERSION");
            if database_existed {
                let previous_version: Option<String> = conn.query_row(
                    "SELECT last_app_version FROM app_meta WHERE id=1",
                    [],
                    |r| r.get::<_, Option<String>>(0),
                ).unwrap_or(None);
                // App-only upgrades still receive a safety backup even when there is
                // no schema migration in that release. Migration upgrades were
                // already backed up above, so avoid creating a duplicate snapshot.
                if !pending_migrations && previous_version.as_deref() != Some(current_version) {
                    backup::create_backup(&conn, &database_path, &backup_dir)?;
                }
            }
            conn.execute("UPDATE app_meta SET last_app_version=?1,updated_at=CURRENT_TIMESTAMP WHERE id=1", [current_version])?;
            let retention: i64 = conn.query_row("SELECT COALESCE(backup_retention_count,14) FROM household_settings WHERE id=1", [], |r| r.get(0)).unwrap_or(14);
            if db::onboarding_complete(&conn).unwrap_or(false) { let _ = backup::create_daily_backup_if_needed(&conn, &database_path, &backup_dir, retention.max(3) as usize); }
            app.manage(AppState { db: Mutex::new(conn), database_path, backup_dir, export_dir, restore_marker });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_bootstrap,
            commands::complete_onboarding,
            commands::get_dashboard_summary,
            commands::create_backup,
            phase3::list_bills,
            phase3::save_bill,
            phase3::get_bill_detail,
            phase3::mark_bill_paid,
            phase3::archive_bill,
            phase3::list_paychecks,
            phase3::save_paycheck,
            phase3::delete_paycheck,
            phase3::list_paycheck_schedules,
            phase3::save_paycheck_schedule,
            phase3::run_scheduler,
            phase3::get_planner,
            phase4::get_dashboard_data,
            phase4::get_spending_view,
            phase4::add_transaction,
            phase4::reconcile_account,
            phase4::get_calendar_data,
            phase4::get_history_data,
            phase5::get_payment_guidance,
            phase5::get_savings_debt_view,
            phase5::save_savings_goal,
            phase5::save_debt,
            phase5::record_savings_contribution,
            phase5::record_debt_payment,
            phase5::archive_savings_goal,
            phase5::archive_debt,
            phase5::get_settings_view,
            phase5::save_settings,
            phase5::open_app_folder,
            phase5::list_backups,
            phase5::request_restore_backup,
            phase5::get_reports_data,
            phase5::export_report_csv,
            updates::check_for_update,
            updates::install_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Household Bills");
}
