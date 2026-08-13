use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_updater::UpdaterExt;

use crate::{backup, AppState};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatusDto {
    pub available: bool,
    pub current_version: String,
    pub version: Option<String>,
}

#[tauri::command]
pub async fn check_for_update(app: AppHandle) -> Result<UpdateStatusDto, String> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let updater = app.updater().map_err(|e| format!("Updater could not start: {e}"))?;
    let update = updater
        .check()
        .await
        .map_err(|e| format!("Update check failed: {e}"))?;

    Ok(match update {
        Some(update) => UpdateStatusDto {
            available: true,
            current_version,
            version: Some(update.version.to_string()),
        },
        None => UpdateStatusDto {
            available: false,
            current_version,
            version: None,
        },
    })
}

#[tauri::command]
pub async fn install_update(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    // Financial data gets a local safety snapshot immediately before every
    // application update. Never hold the database mutex while downloading.
    {
        let conn = state
            .db
            .lock()
            .map_err(|_| "Database lock is unavailable.".to_string())?;
        backup::create_backup(&conn, &state.database_path, &state.backup_dir)
            .map_err(|e| format!("Could not create the pre-update backup: {e}"))?;
    }

    let updater = app.updater().map_err(|e| format!("Updater could not start: {e}"))?;
    let Some(update) = updater
        .check()
        .await
        .map_err(|e| format!("Update check failed: {e}"))?
    else {
        return Err("No newer Household Bills version is available.".to_string());
    };

    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| format!("The update could not be installed: {e}"))?;

    app.restart();
}
