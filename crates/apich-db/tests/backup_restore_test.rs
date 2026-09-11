mod common;
use apich_db::BackupManager;
use apich_db::BackupOptions;
use apich_db::Database;
use apich_db::PostgresConfig;
use apich_db::PostgresContainer;
use common::test_temp_dir;
use sqlx::Row;
use std::time::Duration;

#[tokio::test]
async fn test_postgres_backup_and_restore() {
    let temp_data = test_temp_dir();
    let temp_backup = test_temp_dir();

    let config = PostgresConfig::builder(temp_data.path(), temp_backup.path())
        .container_name("apich-pg-backup-test")
        .host_port(5435)
        .database("test_backup_db")
        .admin_user("postgres")
        .admin_password("admin_backup_pass")
        .selinux_relabel(true)
        .build();

    let container = PostgresContainer::new(config.clone());
    container
        .ensure_running()
        .await
        .expect("Failed to start container");
    container
        .wait_ready(Duration::from_secs(30))
        .await
        .expect("Postgres not ready");

    // 1. Create table and populate data
    let db = Database::connect_admin(&config, "localhost")
        .await
        .expect("Failed to connect admin");

    sqlx::query("DROP TABLE IF EXISTS important_docs")
        .execute(db.admin_pool())
        .await
        .expect("Failed to drop old table");

    sqlx::query("CREATE TABLE important_docs (id INT PRIMARY KEY, title TEXT)")
        .execute(db.admin_pool())
        .await
        .expect("Failed to create table");

    sqlx::query("INSERT INTO important_docs (id, title) VALUES (1, 'Paper A'), (2, 'Thesis B')")
        .execute(db.admin_pool())
        .await
        .expect("Failed to insert rows");

    // 2. Perform database backup
    let backup_mgr = BackupManager::new(&container);
    let backup_info = backup_mgr
        .create_backup(BackupOptions::default())
        .await
        .expect("Failed to create backup");

    assert!(backup_info.host_path.exists());
    assert!(backup_info.size_bytes > 0);

    // 3. List backups
    let backups = backup_mgr.list_backups().expect("Failed to list backups");
    assert!(!backups.is_empty());
    assert_eq!(backups[0].filename, backup_info.filename);

    // 4. Drop table to simulate disaster / data loss
    sqlx::query("DROP TABLE important_docs")
        .execute(db.admin_pool())
        .await
        .expect("Failed to drop table");

    // Close connection before restoring
    drop(db);

    // 5. Restore database from backup
    backup_mgr
        .restore_backup(&backup_info.filename, None)
        .await
        .expect("Failed to restore backup");

    // 6. Reconnect and verify data restoration
    let db2 = Database::connect_admin(&config, "localhost")
        .await
        .expect("Failed to reconnect after restore");

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM important_docs")
        .fetch_one(db2.admin_pool())
        .await
        .expect("Failed to query restored table");
    assert_eq!(count, 2);

    let row = sqlx::query("SELECT title FROM important_docs WHERE id = 1")
        .fetch_one(db2.admin_pool())
        .await
        .unwrap();
    let title: String = row.get("title");
    assert_eq!(title, "Paper A");

    // 7. Test pruning old backups
    let deleted = backup_mgr
        .cleanup_old_backups(0)
        .expect("Failed to cleanup old backups");
    assert!(deleted > 0);

    let remaining_backups = backup_mgr.list_backups().expect("Failed to list backups");
    assert!(remaining_backups.is_empty());

    // Cleanup
    drop(db2);
    container
        .destroy()
        .await
        .expect("Failed to destroy container");
}
