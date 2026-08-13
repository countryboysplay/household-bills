use std::{fs, path::{Path, PathBuf}};
use chrono::Local;
use rusqlite::Connection;
use crate::error::{AppError, AppResult};

fn validate_sqlite_backup(path:&Path)->AppResult<()> {
    let conn=Connection::open(path)?;
    let result:String=conn.query_row("PRAGMA integrity_check",[],|r|r.get(0))?;
    if result.to_lowercase()!="ok" {
        return Err(AppError::Validation(format!("backup failed SQLite integrity check: {result}")));
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct BackupInfo { pub file_name:String, pub created_at:String, pub size_bytes:u64 }

pub fn create_backup(conn: &Connection, database_path: &Path, backup_dir: &Path) -> AppResult<PathBuf> {
    fs::create_dir_all(backup_dir)?;
    conn.execute_batch("PRAGMA wal_checkpoint(FULL);")?;
    let filename = format!("HouseholdBills_{}.sqlite3", Local::now().format("%Y-%m-%d_%H%M%S"));
    let target = backup_dir.join(filename);
    fs::copy(database_path, &target)?;
    Ok(target)
}

pub fn list_backups(backup_dir:&Path)->AppResult<Vec<BackupInfo>>{
    fs::create_dir_all(backup_dir)?;
    let mut items=Vec::new();
    for entry in fs::read_dir(backup_dir)?{
        let entry=entry?; let path=entry.path();
        if path.extension().and_then(|s|s.to_str())!=Some("sqlite3"){continue;}
        let meta=entry.metadata()?; let name=entry.file_name().to_string_lossy().to_string();
        let created=name.strip_prefix("HouseholdBills_").and_then(|s|s.strip_suffix(".sqlite3")).unwrap_or(&name).replace('_'," ");
        items.push(BackupInfo{file_name:name,created_at:created,size_bytes:meta.len()});
    }
    items.sort_by(|a,b|b.file_name.cmp(&a.file_name));
    Ok(items)
}

pub fn prune_backups(backup_dir:&Path,keep:usize)->AppResult<()> {
    let items=list_backups(backup_dir)?;
    for item in items.into_iter().skip(keep.max(1)) { let _=fs::remove_file(backup_dir.join(item.file_name)); }
    Ok(())
}

pub fn create_daily_backup_if_needed(conn:&Connection,database_path:&Path,backup_dir:&Path,retention:usize)->AppResult<Option<PathBuf>>{
    let prefix=format!("HouseholdBills_{}",Local::now().format("%Y-%m-%d"));
    if list_backups(backup_dir)?.iter().any(|b|b.file_name.starts_with(&prefix)){prune_backups(backup_dir,retention)?;return Ok(None);}
    let path=create_backup(conn,database_path,backup_dir)?;prune_backups(backup_dir,retention)?;Ok(Some(path))
}

pub fn schedule_restore(backup_dir:&Path,marker:&Path,file_name:&str)->AppResult<()> {
    if file_name.contains('/')||file_name.contains('\\')||!file_name.ends_with(".sqlite3"){return Err(AppError::Validation("invalid backup file".into()));}
    let source=backup_dir.join(file_name); if !source.is_file(){return Err(AppError::Validation("backup file was not found".into()));}
    validate_sqlite_backup(&source)?;
    fs::write(marker,source.to_string_lossy().as_bytes())?;Ok(())
}

pub fn apply_pending_restore(database_path:&Path,marker:&Path)->AppResult<bool>{
    if !marker.is_file(){return Ok(false);}
    let source=PathBuf::from(fs::read_to_string(marker)?.trim());
    if !source.is_file(){let _=fs::remove_file(marker);return Err(AppError::Validation("pending restore backup no longer exists".into()));}
    validate_sqlite_backup(&source)?;
    let wal=PathBuf::from(format!("{}-wal",database_path.display())); let shm=PathBuf::from(format!("{}-shm",database_path.display()));
    let _=fs::remove_file(&wal);let _=fs::remove_file(&shm);
    fs::copy(&source,database_path)?; let _=fs::remove_file(marker);Ok(true)
}
