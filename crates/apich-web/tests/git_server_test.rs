//! End-to-end integration test for self-hosted Git HTTP server.
//!
//! Spawns a real server, uses the actual `git` binary to clone and push over HTTP
//! with a real PAT to verify that the CGI bridge handles Git client framing correctly.

use apich_db::CreateUserDto;
use apich_db::Database;
use apich_db::PostgresConfig;
use apich_db::PostgresContainer;
use apich_db::UserRole;
use apich_sandbox::SandboxManager;
use apich_web::create_app;
use apich_web::AppState;
use reqwest::StatusCode;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tokio::process::Command;

fn test_temp_dir() -> TempDir {
    let base = std::env::var("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(manifest).join("../../target/tmp")
        });
    let _ = std::fs::create_dir_all(&base);
    tempfile::tempdir_in(&base).unwrap_or_else(|_| tempfile::tempdir().unwrap())
}

async fn run_git(
    args: &[&str],
    cwd: &std::path::Path,
) -> std::process::Output {
    Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("HOME", cwd) // isolate from the real user's git config/credential helpers
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .expect("spawn git")
}

#[tokio::test]
async fn test_git_http_server_real_clone_and_push() {
    let temp_data = test_temp_dir();
    let temp_backup = test_temp_dir();

    let config = PostgresConfig::builder(temp_data.path(), temp_backup.path())
        .container_name("apich-pg-gitserver-test")
        .host_port(5444)
        .database("apich_gitserver_test")
        .admin_user("postgres")
        .admin_password("admin_gitserver_test_pwd")
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

    let db = Arc::new(
        Database::connect_admin(&config, "localhost")
            .await
            .expect("Failed to connect to test Postgres"),
    );
    let _ = db.run_migrations().await;
    let repo = db.repository();
    let _ = sqlx::query(
        "TRUNCATE TABLE users, organizations, oauth_clients, system_settings, invitations CASCADE",
    )
    .execute(db.pool())
    .await;

    let _admin = repo
        .create_user(CreateUserDto {
            username: "dr_gitserver".to_string(),
            email: "gitserver@lab.quantum.org".to_string(),
            password_hash: apich_web::auth::hash_password("SuperSecretPass123!").unwrap(),
            display_name: "Dr. Git Server".to_string(),
            role: Some(UserRole::Admin),
            is_platform_admin: Some(true),
            storage_quota_bytes: None,
        })
        .await
        .expect("Failed to create admin");

    let temp_workspace = test_temp_dir();
    let sandbox_manager = Arc::new(
        SandboxManager::new(temp_workspace.path(), "docker.io/library/alpine:latest")
            .with_selinux(true),
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let server_port = listener.local_addr().expect("local_addr").port();
    let base_url = format!("http://127.0.0.1:{}", server_port);

    let state = AppState::new(
        Arc::clone(&db),
        sandbox_manager,
        temp_workspace.path().to_path_buf(),
        base_url.clone(),
        "test_secret_for_jwt_session_1234567890".to_string(),
    );
    let app = create_app(state.clone());
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    tokio::time::sleep(Duration::from_millis(150)).await;

    let session_client = reqwest::Client::builder()
        .cookie_store(true)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("build client");

    let login_res = session_client
        .post(format!("{}/login", base_url))
        .form(&[
            ("login", "dr_gitserver"),
            ("password", "SuperSecretPass123!"),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(login_res.status(), StatusCode::SEE_OTHER);

    let create_demo_res = session_client
        .post(format!("{}/projects/demo/create", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(create_demo_res.status(), StatusCode::SEE_OTHER);
    let demo_proj_loc = create_demo_res
        .headers()
        .get("location")
        .unwrap()
        .to_str()
        .unwrap();
    let proj_id = demo_proj_loc
        .split('/')
        .nth(2)
        .unwrap()
        .split('?')
        .next()
        .unwrap()
        .to_string();

    // Seed the working `.git` and push it into the bare mirror via the existing "Sync to Git"
    // action (this is what makes the bare mirror non-empty for the clone below).
    let sync_res = session_client
        .post(format!("{}/projects/{}/git-sync", base_url, proj_id))
        .form(&[("message", "Initial export for git server test")])
        .send()
        .await
        .unwrap();
    assert_eq!(sync_res.status(), StatusCode::SEE_OTHER);

    // Generate a real PAT
    let create_pat_res = session_client
        .post(format!("{}/settings/pat/create", base_url))
        .form(&[("name", "git-server-test-token")])
        .send()
        .await
        .unwrap();
    assert_eq!(create_pat_res.status(), StatusCode::SEE_OTHER);
    let pat_location = create_pat_res
        .headers()
        .get("location")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let token_encoded = pat_location
        .split("new_pat_token=")
        .nth(1)
        .expect("token in redirect")
        .split('#')
        .next()
        .unwrap();
    let pat = urlencoding::decode(token_encoded)
        .expect("decode token")
        .into_owned();

    // --- Unauthenticated clone must be rejected ---
    let clone_url_noauth = format!("{}/git/{}.git", base_url, proj_id);
    let unauth_dest = test_temp_dir();
    let unauth_out = run_git(
        &[
            "clone",
            &clone_url_noauth,
            unauth_dest.path().join("x").to_str().unwrap(),
        ],
        unauth_dest.path(),
    )
    .await;
    assert!(!unauth_out.status.success(), "anonymous clone must fail");

    // --- Real `git clone` over HTTP with PAT-as-password Basic auth ---
    let clone_url = format!(
        "http://dr_gitserver:{}@127.0.0.1:{}/git/{}.git",
        pat, server_port, proj_id
    );
    let clone_workdir = test_temp_dir();
    let clone_dest = clone_workdir.path().join("cloned");
    let clone_out = run_git(
        &["clone", &clone_url, clone_dest.to_str().unwrap()],
        clone_workdir.path(),
    )
    .await;
    assert!(
        clone_out.status.success(),
        "git clone failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&clone_out.stdout),
        String::from_utf8_lossy(&clone_out.stderr),
    );
    assert!(clone_dest.join(".git").exists());
    assert!(
        clone_dest.join("slides.typ").exists(),
        "cloned repo should contain the demo project's real files"
    );

    // --- Real `git push` back over HTTP ---
    std::fs::write(
        clone_dest.join("pushed_from_real_git.txt"),
        "hello from a real git client\n",
    )
    .unwrap();
    let add_out = run_git(&["add", "pushed_from_real_git.txt"], &clone_dest).await;
    assert!(add_out.status.success());
    let commit_out = Command::new("git")
        .args([
            "-c",
            "user.email=test@apich.local",
            "-c",
            "user.name=Test",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-m",
            "real git push test",
        ])
        .current_dir(&clone_dest)
        .env("HOME", clone_workdir.path())
        .output()
        .await
        .unwrap();
    assert!(
        commit_out.status.success(),
        "commit failed: {}",
        String::from_utf8_lossy(&commit_out.stderr)
    );

    let push_out = run_git(&["push", "origin", "HEAD"], &clone_dest).await;
    assert!(
        push_out.status.success(),
        "git push failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&push_out.stdout),
        String::from_utf8_lossy(&push_out.stderr),
    );

    // --- Verify the push was actually synced back into the project's real working directory ---
    let repo_after = db.repository();
    let proj_after = repo_after
        .get_project_by_id(uuid::Uuid::parse_str(&proj_id).unwrap())
        .await
        .unwrap()
        .unwrap();
    let pushed_file =
        std::path::PathBuf::from(&proj_after.storage_path).join("pushed_from_real_git.txt");
    // Sync-back runs synchronously inside the push request handler, so it must already be there.
    assert!(
        pushed_file.exists(),
        "pushed file should be checked out into the real project working directory"
    );
    assert_eq!(
        std::fs::read_to_string(&pushed_file).unwrap(),
        "hello from a real git client\n"
    );

    // --- A second, fresh clone must also see the pushed file (proves the bare mirror itself has it) ---
    let second_clone_workdir = test_temp_dir();
    let second_clone_dest = second_clone_workdir.path().join("cloned_again");
    let second_clone_out = run_git(
        &["clone", &clone_url, second_clone_dest.to_str().unwrap()],
        second_clone_workdir.path(),
    )
    .await;
    assert!(
        second_clone_out.status.success(),
        "second clone failed: {}",
        String::from_utf8_lossy(&second_clone_out.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(second_clone_dest.join("pushed_from_real_git.txt")).unwrap(),
        "hello from a real git client\n"
    );
}
