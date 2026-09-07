use crate::container::PostgresContainer;
use crate::error::{DbError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;
use tracing::info;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackupFormat {
    Custom, // pg_dump -Fc (compressed binary, flexible restore)
    Sql,    // pg_dump -Fp (plain SQL text)
    Tar,    // pg_dump -Ft (tar archive)
}

impl BackupFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            BackupFormat::Custom => "dump",
            BackupFormat::Sql => "sql",
            BackupFormat::Tar => "tar",
        }
    }

    pub fn flag(&self) -> &'static str {
        match self {
            BackupFormat::Custom => "-Fc",
            BackupFormat::Sql => "-Fp",
            BackupFormat::Tar => "-Ft",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupOptions {
    pub format: BackupFormat,
    pub target_db: Option<String>,
    pub file_prefix: Option<String>,
    pub compress_level: Option<u32>,
}

impl Default for BackupOptions {
    fn default() -> Self {
        Self {
            format: BackupFormat::Custom,
            target_db: None,
            file_prefix: Some("apich_backup".to_string()),
            compress_level: Some(6),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    pub filename: String,
    pub host_path: PathBuf,
    pub format: BackupFormat,
    pub size_bytes: u64,
    pub created_at: DateTime<Utc>,
}

pub struct BackupManager<'a> {
    container: &'a PostgresContainer,
}

impl<'a> BackupManager<'a> {
    pub fn new(container: &'a PostgresContainer) -> Self {
        Self { container }
    }

    /// Create a database backup using `pg_dump` inside the container
    pub async fn create_backup(&self, opts: BackupOptions) -> Result<BackupInfo> {
        let db = opts
            .target_db
            .unwrap_or_else(|| self.container.config().database.clone());
        let prefix = opts
            .file_prefix
            .unwrap_or_else(|| "apich_backup".to_string());
        let now = Utc::now();
        let timestamp = now.format("%Y%m%d_%H%M%S").to_string();
        let ext = opts.format.extension();
        let filename = format!("{}_{}_{}.{}", prefix, db, timestamp, ext);

        let container_backup_path = format!("/backups/{}", filename);
        let host_backup_path = self.container.config().host_backup_dir.join(&filename);

        info!(
            database = %db,
            filename = %filename,
            format = ?opts.format,
            "Creating PostgreSQL database backup"
        );

        let mut cmd = vec![
            "pg_dump".to_string(),
            "-U".to_string(),
            self.container.config().admin_user.clone(),
            "-d".to_string(),
            db.clone(),
            opts.format.flag().to_string(),
            "-f".to_string(),
            container_backup_path,
        ];

        if let Some(level) = opts.compress_level {
            cmd.push("-Z".to_string());
            cmd.push(level.to_string());
        }

        let exec_opts = apich_sandbox::ExecOptions::new(cmd.iter().map(|s| s.as_str()))
            .env("PGPASSWORD", &self.container.config().admin_password);

        let res = self.container.exec_with_options(&exec_opts).await?;
        res.ensure_success(&self.container.config().container_name)?;

        // Verify file written to host backup mount
        if !host_backup_path.exists() {
            return Err(DbError::BackupFailed(format!(
                "Backup file '{}' was not found on host filesystem after pg_dump",
                host_backup_path.display()
            )));
        }

        let metadata = fs::metadata(&host_backup_path)?;
        let size_bytes = metadata.len();

        info!(
            filename = %filename,
            size_bytes = size_bytes,
            "Database backup completed successfully"
        );

        Ok(BackupInfo {
            filename,
            host_path: host_backup_path,
            format: opts.format,
            size_bytes,
            created_at: now,
        })
    }

    /// Restore database from an existing backup file
    pub async fn restore_backup(&self, filename: &str, target_db: Option<&str>) -> Result<()> {
        let host_backup_path = self.container.config().host_backup_dir.join(filename);
        if !host_backup_path.exists() {
            return Err(DbError::BackupNotFound(host_backup_path));
        }

        let db = target_db.unwrap_or(&self.container.config().database);
        let container_backup_path = format!("/backups/{}", filename);

        info!(
            filename = %filename,
            database = %db,
            "Restoring PostgreSQL database from backup"
        );

        let cmd: Vec<&str> = if filename.ends_with(".dump") || filename.ends_with(".tar") {
            // Restore custom or tar archive using pg_restore
            vec![
                "pg_restore",
                "-U",
                &self.container.config().admin_user,
                "-d",
                db,
                "--clean",
                "--if-exists",
                &container_backup_path,
            ]
        } else {
            // Restore plain SQL script using psql
            vec![
                "psql",
                "-U",
                &self.container.config().admin_user,
                "-d",
                db,
                "-f",
                &container_backup_path,
            ]
        };

        let exec_opts = apich_sandbox::ExecOptions::new(cmd)
            .env("PGPASSWORD", &self.container.config().admin_password);

        let res = self.container.exec_with_options(&exec_opts).await?;
        res.ensure_success(&self.container.config().container_name)?;
        info!(filename = %filename, "Database restore completed successfully");
        Ok(())
    }

    /// List all existing backups stored in the host backup directory
    pub fn list_backups(&self) -> Result<Vec<BackupInfo>> {
        let backup_dir = &self.container.config().host_backup_dir;
        if !backup_dir.exists() {
            return Ok(Vec::new());
        }

        let mut list = Vec::new();
        for entry in fs::read_dir(backup_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let filename = entry.file_name().to_string_lossy().to_string();
                let format = if filename.ends_with(".dump") {
                    BackupFormat::Custom
                } else if filename.ends_with(".sql") {
                    BackupFormat::Sql
                } else if filename.ends_with(".tar") {
                    BackupFormat::Tar
                } else {
                    continue;
                };

                let meta = entry.metadata()?;
                let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                let created_at = DateTime::<Utc>::from(modified);

                list.push(BackupInfo {
                    filename,
                    host_path: path,
                    format,
                    size_bytes: meta.len(),
                    created_at,
                });
            }
        }

        list.sort_by_key(|a| std::cmp::Reverse(a.created_at));
        Ok(list)
    }

    /// Delete a specific backup file
    pub fn delete_backup(&self, filename: &str) -> Result<()> {
        let path = self.container.config().host_backup_dir.join(filename);
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    /// Prune old backups, keeping only the latest `keep_latest` backups
    pub fn cleanup_old_backups(&self, keep_latest: usize) -> Result<usize> {
        let backups = self.list_backups()?;
        let mut deleted = 0;
        if backups.len() > keep_latest {
            for b in &backups[keep_latest..] {
                self.delete_backup(&b.filename)?;
                deleted += 1;
            }
        }
        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backup_format_attributes() {
        assert_eq!(BackupFormat::Custom.extension(), "dump");
        assert_eq!(BackupFormat::Sql.extension(), "sql");
        assert_eq!(BackupFormat::Tar.extension(), "tar");

        assert_eq!(BackupFormat::Custom.flag(), "-Fc");
        assert_eq!(BackupFormat::Sql.flag(), "-Fp");
        assert_eq!(BackupFormat::Tar.flag(), "-Ft");
    }

    #[test]
    fn test_backup_options_default() {
        let opts = BackupOptions::default();
        assert_eq!(opts.format, BackupFormat::Custom);
        assert_eq!(opts.file_prefix.as_deref(), Some("apich_backup"));
        assert_eq!(opts.compress_level, Some(6));
    }
}
