use crate::container::PostgresContainer;
use crate::error::DbError;
use crate::error::Result;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;
use tracing::info;

/// Supported backup file formats for PostgreSQL dumps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackupFormat {
    /// Compressed binary format (`pg_dump -Fc`), suitable for flexible restores.
    Custom,
    /// Plain text SQL script (`pg_dump -Fp`).
    Sql,
    /// Uncompressed tar archive format (`pg_dump -Ft`).
    Tar,
}

impl BackupFormat {
    /// Returns the standard file extension for the backup format.
    #[must_use]
    pub const fn extension(&self) -> &'static str {
        match self {
            | Self::Custom => "dump",
            | Self::Sql => "sql",
            | Self::Tar => "tar",
        }
    }

    /// Returns the command line flag used by `pg_dump` for this format.
    #[must_use]
    pub const fn flag(&self) -> &'static str {
        match self {
            | Self::Custom => "-Fc",
            | Self::Sql => "-Fp",
            | Self::Tar => "-Ft",
        }
    }
}

/// Options configuring a database backup execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupOptions {
    /// Backup format to generate.
    pub format: BackupFormat,
    /// Target database name (defaults to configured primary database).
    pub target_db: Option<String>,
    /// Prefix prepended to the backup filename.
    pub file_prefix: Option<String>,
    /// Compression level from 0 to 9.
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

/// Metadata describing a completed backup file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    /// Base filename of the backup.
    pub filename: String,
    /// Path to the backup file on the host filesystem.
    pub host_path: PathBuf,
    /// Format of the backup file.
    pub format: BackupFormat,
    /// Size of the backup file in bytes.
    pub size_bytes: u64,
    /// UTC timestamp when the backup was created.
    pub created_at: DateTime<Utc>,
}

/// Manager for database backups and restores via containerized PostgreSQL utilities.
pub struct BackupManager<'a> {
    container: &'a PostgresContainer,
}

impl<'a> BackupManager<'a> {
    /// Creates a new backup manager bound to a PostgreSQL container instance.
    #[must_use]
    pub const fn new(container: &'a PostgresContainer) -> Self {
        Self { container }
    }

    /// Create a database backup using `pg_dump` inside the container
    ///
    /// # Errors
    /// Returns an error if executing `pg_dump` fails or if reading the backup file fails.
    pub async fn create_backup(
        &self,
        opts: BackupOptions,
    ) -> Result<BackupInfo> {
        let db = opts
            .target_db
            .unwrap_or_else(|| self.container.config().database.clone());
        let prefix = opts
            .file_prefix
            .unwrap_or_else(|| "apich_backup".to_string());
        let now = Utc::now();
        let timestamp = now.format("%Y%m%d_%H%M%S").to_string();
        let ext = opts.format.extension();
        let filename = format!("{prefix}_{db}_{timestamp}.{ext}");

        let container_backup_path = format!("/backups/{filename}");
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

        let exec_opts =
            apich_sandbox::ExecOptions::new(cmd.iter().map(std::string::String::as_str))
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
    ///
    /// # Errors
    /// Returns an error if the backup file is not found, executing the restore command fails, or command exits with an error code.
    pub async fn restore_backup(
        &self,
        filename: &str,
        target_db: Option<&str>,
    ) -> Result<()> {
        let host_backup_path = self.container.config().host_backup_dir.join(filename);
        if !host_backup_path.exists() {
            return Err(DbError::BackupNotFound(host_backup_path));
        }

        let db = target_db.unwrap_or(&self.container.config().database);
        let container_backup_path = format!("/backups/{filename}");

        info!(
            filename = %filename,
            database = %db,
            "Restoring PostgreSQL database from backup"
        );

        let ext = Path::new(filename)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        let cmd: Vec<&str> = if ext.eq_ignore_ascii_case("dump") || ext.eq_ignore_ascii_case("tar")
        {
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
    ///
    /// # Errors
    /// Returns an error if reading the backup directory or file metadata fails.
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
                let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
                let format = if ext.eq_ignore_ascii_case("dump") {
                    BackupFormat::Custom
                } else if ext.eq_ignore_ascii_case("sql") {
                    BackupFormat::Sql
                } else if ext.eq_ignore_ascii_case("tar") {
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
    ///
    /// # Errors
    /// Returns an error if removing the backup file from disk fails.
    pub fn delete_backup(
        &self,
        filename: &str,
    ) -> Result<()> {
        let path = self.container.config().host_backup_dir.join(filename);
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    /// Prune old backups, keeping only the latest `keep_latest` backups
    ///
    /// # Errors
    /// Returns an error if listing or deleting backup files fails.
    pub fn cleanup_old_backups(
        &self,
        keep_latest: usize,
    ) -> Result<usize> {
        let backups = self.list_backups()?;
        let mut deleted: usize = 0;
        if let Some(to_delete) = backups.get(keep_latest..) {
            for b in to_delete {
                self.delete_backup(&b.filename)?;
                deleted = deleted.saturating_add(1);
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
