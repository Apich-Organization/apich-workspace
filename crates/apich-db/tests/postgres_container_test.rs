mod common;
use apich_db::{Database, PostgresConfig, PostgresContainer};
use common::test_temp_dir;
use sqlx::Row;
use std::time::Duration;

#[tokio::test]
async fn test_postgres_container_lifecycle_and_persistence() {
    let temp_data = test_temp_dir();
    let temp_backup = test_temp_dir();

    let config = PostgresConfig::builder(temp_data.path(), temp_backup.path())
        .container_name("apich-pg-lifecycle-test")
        .host_port(5433)
        .database("test_lifecycle_db")
        .admin_user("postgres")
        .admin_password("test_secret_pass")
        .selinux_relabel(true)
        .build();

    let container = PostgresContainer::new(config.clone());

    // 1. Ensure container is running
    container
        .ensure_running()
        .await
        .expect("Failed to start Postgres container");

    // 2. Wait until healthy
    container
        .wait_ready(Duration::from_secs(30))
        .await
        .expect("PostgreSQL failed to become ready");

    assert!(container.is_running().await.unwrap());

    // 3. Connect via SQLx and write test data
    let db = Database::connect_admin(&config, "localhost")
        .await
        .expect("Failed to connect admin to Postgres");
    db.ping().await.expect("Failed to ping database");

    sqlx::query("CREATE TABLE persistence_test (id INT PRIMARY KEY, val TEXT)")
        .execute(db.admin_pool())
        .await
        .expect("Failed to create table");

    sqlx::query("INSERT INTO persistence_test (id, val) VALUES (1, 'data_persisted_on_host')")
        .execute(db.admin_pool())
        .await
        .expect("Failed to insert row");

    // Drop connection pool before restarting container
    drop(db);

    // 4. Restart container to test host volume data persistence
    container
        .restart(5)
        .await
        .expect("Failed to restart Postgres container");

    container
        .wait_ready(Duration::from_secs(30))
        .await
        .expect("Postgres failed to become ready after restart");

    // 5. Reconnect and verify data was persisted to host data directory
    let db2 = Database::connect_admin(&config, "localhost")
        .await
        .expect("Failed to reconnect after restart");

    let row = sqlx::query("SELECT val FROM persistence_test WHERE id = 1")
        .fetch_one(db2.admin_pool())
        .await
        .expect("Failed to query persisted table");

    let val: String = row.get("val");
    assert_eq!(val, "data_persisted_on_host");

    // 6. Cleanup
    drop(db2);
    container
        .destroy()
        .await
        .expect("Failed to destroy test container");
}
