//! Real end-to-end test for apich-vcs's own remote protocol (`/vcs-remote/:id/bundle`): a real
//! Postgres-backed server, real PAT-based auth (no session cookie), and a real second on-disk
//! clone that pushes a new snapshot back and is verified to actually advance the server's history.

use apich_db::CreateUserDto;
use apich_db::Database;
use apich_db::PostgresConfig;
use apich_db::PostgresContainer;
use apich_db::UserRole;
use apich_sandbox::SandboxManager;
use apich_vcs::ProjectVcs;
use apich_web::create_app;
use apich_web::AppState;
use reqwest::StatusCode;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;

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

#[tokio::test]
async fn test_vcs_remote_bundle_clone_and_pat_authenticated_push() {
    let temp_data = test_temp_dir();
    let temp_backup = test_temp_dir();

    let config = PostgresConfig::builder(temp_data.path(), temp_backup.path())
        .container_name("apich-pg-vcsremote-test")
        .host_port(5443)
        .database("apich_vcsremote_test")
        .admin_user("postgres")
        .admin_password("admin_vcsremote_test_pwd")
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
            username: "dr_bundle".to_string(),
            email: "bundle@lab.quantum.org".to_string(),
            password_hash: apich_web::auth::hash_password("SuperSecretPass123!").unwrap(),
            display_name: "Dr. Bundle".to_string(),
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
        .form(&[("login", "dr_bundle"), ("password", "SuperSecretPass123!")])
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

    // --- Generate a real PAT ---
    let create_pat_res = session_client
        .post(format!("{}/settings/pat/create", base_url))
        .form(&[("name", "vcs-remote-test-token")])
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
    assert!(pat.starts_with("apat_"), "unexpected token shape: {pat}");

    // --- Unauthenticated requests must be rejected ---
    let anon_client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let anon_get = anon_client
        .get(format!("{}/vcs-remote/{}/bundle", base_url, proj_id))
        .send()
        .await
        .unwrap();
    assert_eq!(anon_get.status(), StatusCode::UNAUTHORIZED);
    let anon_post = anon_client
        .post(format!("{}/vcs-remote/{}/bundle", base_url, proj_id))
        .body(vec![0u8; 4])
        .send()
        .await
        .unwrap();
    assert_eq!(anon_post.status(), StatusCode::UNAUTHORIZED);

    // --- "Clone": download the bundle using ONLY the PAT (no session cookie at all) ---
    let pat_client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let clone_res = pat_client
        .get(format!("{}/vcs-remote/{}/bundle", base_url, proj_id))
        .bearer_auth(&pat)
        .send()
        .await
        .unwrap();
    assert_eq!(clone_res.status(), StatusCode::OK);
    let bundle_bytes = clone_res.bytes().await.unwrap().to_vec();
    assert!(
        bundle_bytes.starts_with(&[0x1f, 0x8b]),
        "response should be a real gzip stream"
    );

    let client_temp = test_temp_dir();
    let client_root = client_temp.path().join("cloned_via_pat");
    let client_vcs =
        ProjectVcs::import_bundle(&bundle_bytes[..], &client_root).expect("import cloned bundle");
    let original_head = client_vcs.head_snapshot().unwrap().unwrap().id;

    // --- Make real local progress in the clone, then push it back using ONLY the PAT ---
    std::fs::write(
        client_root.join("pushed_via_pat.txt"),
        "pushed from a PAT-authenticated client\n",
    )
    .unwrap();
    let pushed_snap = client_vcs
        .snapshot("Pushed via PAT-authenticated apich remote push")
        .unwrap();
    assert_ne!(pushed_snap.id, original_head);

    let mut push_bytes = Vec::new();
    client_vcs
        .export_bundle(&mut push_bytes, apich_vcs::BundleOptions::default())
        .unwrap();

    let push_res = pat_client
        .post(format!("{}/vcs-remote/{}/bundle", base_url, proj_id))
        .bearer_auth(&pat)
        .body(push_bytes)
        .send()
        .await
        .unwrap();
    assert_eq!(push_res.status(), StatusCode::OK);
    let push_json: Value = push_res.json().await.unwrap();
    let accepted = push_json["accepted_branches"].as_array().unwrap();
    assert!(
        accepted.iter().any(|b| b == "main"),
        "expected main branch to be accepted, got: {push_json}"
    );
    assert!(push_json["rejected_branches"]
        .as_array()
        .unwrap()
        .is_empty());

    // --- Verify the server's own history actually advanced (re-download and check) ---
    let verify_res = pat_client
        .get(format!("{}/vcs-remote/{}/bundle", base_url, proj_id))
        .bearer_auth(&pat)
        .send()
        .await
        .unwrap();
    assert_eq!(verify_res.status(), StatusCode::OK);
    let verify_bytes = verify_res.bytes().await.unwrap().to_vec();

    let verify_temp = test_temp_dir();
    let verify_root = verify_temp.path().join("post_push_check");
    let verify_vcs = ProjectVcs::import_bundle(&verify_bytes[..], &verify_root).unwrap();
    assert_eq!(
        verify_vcs.head_snapshot().unwrap().unwrap().id,
        pushed_snap.id,
        "server's HEAD must equal the snapshot that was pushed"
    );
    assert_eq!(
        std::fs::read_to_string(verify_root.join("pushed_via_pat.txt")).unwrap(),
        "pushed from a PAT-authenticated client\n",
    );
}
