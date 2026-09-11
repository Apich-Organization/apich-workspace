mod common;
use apich_db::Database;
use apich_db::PermissionManager;
use apich_db::PostgresConfig;
use apich_db::PostgresContainer;
use common::test_temp_dir;
use std::time::Duration;

#[tokio::test]
async fn test_postgres_least_privilege_permissions() {
    let temp_data = test_temp_dir();
    let temp_backup = test_temp_dir();

    let config = PostgresConfig::builder(temp_data.path(), temp_backup.path())
        .container_name("apich-pg-permissions-test")
        .host_port(5434)
        .database("test_perms_db")
        .admin_user("postgres")
        .admin_password("admin_strong_secret")
        .app_user("apich_app_tester")
        .app_password("app_restricted_secret")
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

    // 1. Connect admin and set up permissions
    let mut db = Database::connect_admin(&config, "localhost")
        .await
        .expect("Failed to connect admin");

    db.setup_permissions(&config)
        .await
        .expect("Failed to setup least privilege");

    // 2. Connect with app role
    db.init_app_pool(&config, "localhost")
        .await
        .expect("Failed to connect app pool");

    let app_pool = db.app_pool().expect("App pool missing");

    // 3. Verify non-superuser attributes
    let ok = PermissionManager::verify_permissions(app_pool, &config.app_user)
        .await
        .expect("Permission verification failed");
    assert!(ok);

    // 4. Verify app user can CRUD tables in schema public
    sqlx::query("CREATE TABLE public.app_test (id INT PRIMARY KEY, name TEXT)")
        .execute(app_pool)
        .await
        .expect("App user should be able to create table in public schema");

    sqlx::query("INSERT INTO public.app_test (id, name) VALUES (1, 'hello')")
        .execute(app_pool)
        .await
        .expect("App user should be able to insert data");

    // 5. Verify app user CANNOT perform superuser operations (e.g. create database)
    let bad_res = sqlx::query("CREATE DATABASE unauthorized_db")
        .execute(app_pool)
        .await;
    assert!(
        bad_res.is_err(),
        "Application user must NOT be able to create databases (least privilege violation!)"
    );

    // Cleanup
    drop(db);
    container
        .destroy()
        .await
        .expect("Failed to destroy container");
}
