use std::fs;
use std::path::Path;

use crate::stats;

pub fn daily_backup(db_path: &Path) -> Result<(), String> {
    let backup_dir = db_path
        .parent()
        .ok_or("Cannot determine database directory")?
        .join("backups");

    fs::create_dir_all(&backup_dir).map_err(|e| format!("Cannot create backup dir: {e}"))?;

    let today_start = stats::day_start(stats::now_ts());
    let today_file = backup_dir.join(format!("chronos_{today_start}.db"));

    if today_file.exists() {
        return Ok(());
    }

    let tmp = backup_dir.join("chronos_backup_tmp.db");
    fs::copy(db_path, &tmp).map_err(|e| format!("Backup copy failed: {e}"))?;
    fs::rename(&tmp, &today_file).map_err(|e| format!("Backup rename failed: {e}"))?;

    Ok(())
}

pub fn prune_backups(db_path: &Path, keep_days: u64) -> Result<(), String> {
    let backup_dir = db_path
        .parent()
        .ok_or("Cannot determine database directory")?
        .join("backups");

    if !backup_dir.exists() {
        return Ok(());
    }

    let cutoff = stats::now_ts() - (keep_days as i64 * 86400);

    for entry in fs::read_dir(&backup_dir).map_err(|e| format!("Cannot read backup dir: {e}"))? {
        let entry = entry.map_err(|e| format!("Cannot read entry: {e}"))?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("db") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                if let Some(ts_str) = stem.strip_prefix("chronos_") {
                    if let Ok(ts) = ts_str.parse::<i64>() {
                        if ts < cutoff {
                            let _ = fs::remove_file(&path);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backup_creates_file() {
        let dir = std::env::temp_dir().join(format!("chronos_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let db_path = dir.join("test.db");
        std::fs::write(&db_path, b"content").unwrap();

        daily_backup(&db_path).unwrap();
        let backup_dir = dir.join("backups");
        assert!(backup_dir.exists());
        let count = fs::read_dir(&backup_dir).unwrap().count();
        assert_eq!(count, 1, "Should have exactly one backup file");

        daily_backup(&db_path).unwrap();
        let count = fs::read_dir(&backup_dir).unwrap().count();
        assert_eq!(count, 1, "Should not create duplicate daily backup");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_prune_old_backups() {
        let dir = std::env::temp_dir().join(format!("chronos_prune_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let backup_dir = dir.join("backups");
        fs::create_dir_all(&backup_dir).unwrap();

        let old_ts = stats::now_ts() - (30 * 86400);
        let recent_ts = stats::now_ts() - 3600;

        std::fs::write(backup_dir.join(format!("chronos_{old_ts}.db")), b"old").unwrap();
        std::fs::write(
            backup_dir.join(format!("chronos_{recent_ts}.db")),
            b"recent",
        )
        .unwrap();

        let db_path = dir.join("test.db");
        std::fs::write(&db_path, b"content").unwrap();

        prune_backups(&db_path, 14).unwrap();

        let remaining: Vec<_> = fs::read_dir(&backup_dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();

        assert_eq!(remaining.len(), 1, "Should keep only the recent backup");
        assert!(
            remaining[0]
                .to_string_lossy()
                .contains(&recent_ts.to_string()),
            "Should keep the recent backup"
        );

        let _ = fs::remove_dir_all(&dir);
    }
}
