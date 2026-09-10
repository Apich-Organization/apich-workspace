use apich_db::{
    CreateOAuthClientDto, CreateOrganizationDto, CreateTeamDto, CreateUserDto, Database,
    PostgresConfig, PostgresContainer, UpdateOrganizationDto, UpdateTeamDto, UserRole,
};
use apich_sandbox::SandboxManager;
use apich_web::{create_app, AppState};
use base64::prelude::*;
use p256::ecdsa::{signature::Signer, Signature, SigningKey};
use reqwest::StatusCode;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{path::PathBuf, sync::Arc, time::Duration};
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
async fn test_fullstack_web_e2e_lifecycle() {
    // 0. Setup PostgreSQL 18 container & migrations
    let temp_data = test_temp_dir();
    let temp_backup = test_temp_dir();

    let config = PostgresConfig::builder(temp_data.path(), temp_backup.path())
        .container_name("apich-pg-web-test")
        .host_port(5440)
        .database("apich_web_test")
        .admin_user("postgres")
        .admin_password("admin_web_test_pwd")
        .selinux_relabel(true)
        .build();

    let container = PostgresContainer::new(config.clone());
    container
        .ensure_running()
        .await
        .expect("Failed to start Postgres container");
    container
        .wait_ready(Duration::from_secs(30))
        .await
        .expect("Postgres not ready");

    let db = Arc::new(
        Database::connect_admin(&config, "localhost")
            .await
            .expect("Failed to connect admin"),
    );

    db.run_migrations()
        .await
        .expect("Failed to run migrations");

    // Clean tables for reproducible test runs
    let _ = sqlx::query("TRUNCATE TABLE users, organizations, oauth_clients, system_settings, invitations CASCADE")
        .execute(db.pool())
        .await;

    let repo = db.repository();

    let _ = sqlx::query("INSERT INTO system_settings (id, registration_mode) VALUES (1, 'open') ON CONFLICT (id) DO UPDATE SET registration_mode = 'open'")
        .execute(db.pool())
        .await;

    // Create Platform Admin: Alice
    let alice = repo
        .create_user(CreateUserDto {
            username: "alice_admin".to_string(),
            email: "alice@apich.org".to_string(),
            password_hash: apich_web::auth::hash_password("AliceAdmin123!").unwrap(),
            display_name: "Alice Platform Admin".to_string(),
            role: Some(UserRole::Admin),
            is_platform_admin: Some(true),
            storage_quota_bytes: None,
        })
        .await
        .expect("Failed to seed platform admin Alice");

    repo.update_system_settings(apich_db::UpdateSystemSettingsDto {
        registration_mode: Some("open".to_string()),
        ..Default::default()
    })
    .await
    .expect("Failed to set open registration mode");

    // 1. Setup Sandbox Manager & Web Application Server
    let temp_workspace = test_temp_dir();
    let sandbox_manager = Arc::new(
        SandboxManager::new(temp_workspace.path(), "docker.io/library/alpine:latest")
            .with_selinux(true),
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind ephemeral TCP port");
    let server_port = listener.local_addr().expect("Failed to get local port").port();
    let base_url = format!("http://127.0.0.1:{}", server_port);

    let state = AppState::new(
        Arc::clone(&db),
        sandbox_manager,
        temp_workspace.path().to_path_buf(),
        base_url.clone(),
        "web_e2e_jwt_secret_token_1234567890_test_only".to_string(),
    );

    let app = create_app(state.clone());

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Give server a moment to start accepting connections
    tokio::time::sleep(Duration::from_millis(150)).await;

    // HTTP Client with cookie jar and manual redirect handling
    let client = reqwest::Client::builder()
        .cookie_store(true)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Failed to build HTTP client");

    // ========================================================================
    // TEST 1: User Registration, Duplicate Handling, Login, Session & Profile
    // ========================================================================
    println!("--- Running Test 1: User Registration & Session Auth ---");

    // Register Bob
    let reg_res = client
        .post(format!("{}/api/auth/register", base_url))
        .json(&json!({
            "username": "bob_researcher",
            "email": "bob@apich.org",
            "password": "BobSecretPassword123!",
            "display_name": "Bob Researcher"
        }))
        .send()
        .await
        .expect("Failed to send register request");

    assert_eq!(reg_res.status(), StatusCode::OK);
    let reg_body: Value = reg_res.json().await.unwrap();
    assert_eq!(reg_body["user"]["username"], "bob_researcher");
    assert_eq!(reg_body["user"]["email"], "bob@apich.org");

    // Duplicate registration attempt (same username) -> 409 Conflict
    let dup_res = client
        .post(format!("{}/api/auth/register", base_url))
        .json(&json!({
            "username": "bob_researcher",
            "email": "other@apich.org",
            "password": "AnyPassword123!",
            "display_name": "Other Bob"
        }))
        .send()
        .await
        .expect("Failed to send dup register request");
    assert_eq!(dup_res.status(), StatusCode::CONFLICT);

    // Login as Bob
    let login_res = client
        .post(format!("{}/api/auth/login", base_url))
        .json(&json!({
            "login": "bob_researcher",
            "password": "BobSecretPassword123!"
        }))
        .send()
        .await
        .expect("Failed to send login request");

    assert_eq!(login_res.status(), StatusCode::OK);
    assert!(login_res.headers().contains_key("set-cookie"));

    // Check Current Profile (/api/auth/me) with session cookie
    let me_res = client
        .get(format!("{}/api/auth/me", base_url))
        .send()
        .await
        .expect("Failed to fetch /api/auth/me");
    assert_eq!(me_res.status(), StatusCode::OK);
    let me_body: Value = me_res.json().await.unwrap();
    assert_eq!(me_body["username"], "bob_researcher");
    let bob_id = me_body["id"].as_str().unwrap().to_string();

    // Logout
    let logout_res = client
        .post(format!("{}/api/auth/logout", base_url))
        .send()
        .await
        .expect("Failed to logout");
    assert_eq!(logout_res.status(), StatusCode::OK);

    // Profile request after logout -> 401 Unauthorized
    let me_unauth = client
        .get(format!("{}/api/auth/me", base_url))
        .send()
        .await
        .expect("Failed to fetch /api/auth/me");
    assert_eq!(me_unauth.status(), StatusCode::UNAUTHORIZED);

    // ========================================================================
    // TEST 2: FIDO2 / WebAuthn Passkey Registration & Authentication Ceremony
    // ========================================================================
    println!("--- Running Test 2: FIDO2 / Passkey Ceremony ---");

    // Re-login as Bob to enroll passkey
    let _ = client
        .post(format!("{}/api/auth/login", base_url))
        .json(&json!({
            "login": "bob_researcher",
            "password": "BobSecretPassword123!"
        }))
        .send()
        .await
        .unwrap();

    // Start Passkey Registration
    let challenge_res = client
        .post(format!("{}/api/auth/fido2/register/challenge", base_url))
        .send()
        .await
        .expect("Failed to start passkey registration");
    assert_eq!(challenge_res.status(), StatusCode::OK);
    let challenge_body: Value = challenge_res.json().await.unwrap();
    let reg_challenge = challenge_body["challenge"].as_str().unwrap().to_string();
    assert!(!reg_challenge.is_empty());

    // Generate P-256 ECDSA key pair
    let signing_key = SigningKey::from_slice(&[42u8; 32]).unwrap();
    let verifying_key = signing_key.verifying_key();
    let pub_key_bytes = verifying_key.to_sec1_bytes();
    let pub_key_b64 = BASE64_STANDARD.encode(&pub_key_bytes);
    let credential_id = "fido2-bob-yubikey-sec01".to_string();

    // Finish Passkey Registration
    let enroll_res = client
        .post(format!("{}/api/auth/fido2/register/finish", base_url))
        .json(&json!({
            "credential_id": credential_id,
            "public_key_base64": pub_key_b64,
            "device_name": "Bob Enterprise YubiKey"
        }))
        .send()
        .await
        .expect("Failed to finish passkey registration");
    assert_eq!(enroll_res.status(), StatusCode::OK);

    // Logout to test authentication ceremony from cold state
    let _ = client.post(format!("{}/api/auth/logout", base_url)).send().await.unwrap();

    // Start Passkey Authentication Ceremony
    let auth_start_res = client
        .post(format!("{}/api/auth/fido2/auth/challenge", base_url))
        .send()
        .await
        .expect("Failed to request auth challenge");
    assert_eq!(auth_start_res.status(), StatusCode::OK);
    let auth_start_body: Value = auth_start_res.json().await.unwrap();
    let auth_challenge = auth_start_body["challenge"].as_str().unwrap().to_string();

    // Construct WebAuthn assertion data
    let client_data_json = json!({
        "type": "webauthn.get",
        "challenge": auth_challenge,
        "origin": "http://localhost:8080"
    })
    .to_string()
    .into_bytes();

    let auth_data = vec![1u8; 37]; // 37-byte authenticator data
    let mut signed_data = Vec::new();
    signed_data.extend_from_slice(&auth_data);
    let client_hash = Sha256::digest(&client_data_json);
    signed_data.extend_from_slice(&client_hash);

    let signature: Signature = signing_key.sign(&signed_data);
    let sig_der = signature.to_der();

    // Finish Passkey Authentication
    let auth_finish_res = client
        .post(format!("{}/api/auth/fido2/auth/finish", base_url))
        .json(&json!({
            "credential_id": credential_id,
            "challenge": auth_challenge,
            "client_data_json_base64": BASE64_STANDARD.encode(&client_data_json),
            "auth_data_base64": BASE64_STANDARD.encode(&auth_data),
            "signature_base64": BASE64_STANDARD.encode(sig_der.as_bytes())
        }))
        .send()
        .await
        .expect("Failed to finish passkey authentication");

    assert_eq!(auth_finish_res.status(), StatusCode::OK);
    let auth_finish_body: Value = auth_finish_res.json().await.unwrap();
    assert_eq!(auth_finish_body["user"]["username"], "bob_researcher");

    // Check we are now fully logged in
    let check_logged_in = client
        .get(format!("{}/api/auth/me", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(check_logged_in.status(), StatusCode::OK);

    // ========================================================================
    // TEST 3: Single Sign-On (SSO) & OIDC Provider Flow
    // ========================================================================
    println!("--- Running Test 3: SSO & OIDC Provider Flow ---");

    // OIDC Discovery
    let discovery_res = client
        .get(format!("{}/.well-known/openid-configuration", base_url))
        .send()
        .await
        .expect("Failed to fetch openid configuration");
    assert_eq!(discovery_res.status(), StatusCode::OK);
    let discovery_body: Value = discovery_res.json().await.unwrap();
    assert_eq!(discovery_body["issuer"], base_url);
    assert!(discovery_body["authorization_endpoint"].as_str().unwrap().ends_with("/oauth/authorize"));
    assert!(discovery_body["token_endpoint"].as_str().unwrap().ends_with("/oauth/token"));
    assert!(discovery_body["userinfo_endpoint"].as_str().unwrap().ends_with("/oauth/userinfo"));

    // JWKS
    let jwks_res = client
        .get(format!("{}/oauth/jwks.json", base_url))
        .send()
        .await
        .expect("Failed to fetch JWKS");
    assert_eq!(jwks_res.status(), StatusCode::OK);
    let jwks_body: Value = jwks_res.json().await.unwrap();
    assert!(jwks_body["keys"].as_array().is_some());

    // Register OAuth2 Client in Database
    let bob_uuid = uuid::Uuid::parse_str(&bob_id).unwrap();
    let oauth_client = state
        .db
        .repository()
        .create_oauth_client(CreateOAuthClientDto {
            name: "External Research Portal".to_string(),
            client_id: "ext-portal-client-id".to_string(),
            client_secret_hash: Some("ext_secret_hash".to_string()),
            redirect_uris: vec!["http://client.local/callback".to_string()],
            is_confidential: false,
        })
        .await
        .expect("Failed to create OAuth client");

    // Initiate /oauth/authorize (as authenticated Bob)
    let authz_res = client
        .get(format!(
            "{}/oauth/authorize?response_type=code&client_id={}&redirect_uri={}&scope=openid%20profile%20email&state=test_state_1234",
            base_url,
            oauth_client.client_id,
            urlencoding::encode("http://client.local/callback")
        ))
        .send()
        .await
        .expect("Failed to request authorization");

    // Expect 302 or 303 Redirect to callback URL with auth code
    assert!(authz_res.status().is_redirection());
    let location = authz_res
        .headers()
        .get("location")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(location.starts_with("http://client.local/callback?code="));
    assert!(location.contains("&state=test_state_1234"));

    // Extract authorization code from location URL
    let code_param = location
        .split("?code=")
        .nth(1)
        .unwrap()
        .split('&')
        .next()
        .unwrap();

    // Exchange code for tokens at /oauth/token
    let token_res = client
        .post(format!("{}/oauth/token", base_url))
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code_param),
            ("client_id", &oauth_client.client_id),
            ("redirect_uri", "http://client.local/callback"),
        ])
        .send()
        .await
        .expect("Failed to exchange code at /oauth/token");

    assert_eq!(token_res.status(), StatusCode::OK);
    let token_body: Value = token_res.json().await.unwrap();
    let access_token = token_body["access_token"].as_str().unwrap().to_string();
    let id_token = token_body["id_token"].as_str().unwrap().to_string();
    assert_eq!(token_body["token_type"], "Bearer");
    assert!(!access_token.is_empty());
    assert!(!id_token.is_empty());

    // Fetch UserInfo with Bearer access token
    let userinfo_res = client
        .get(format!("{}/oauth/userinfo", base_url))
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
        .expect("Failed to fetch /oauth/userinfo");

    assert_eq!(userinfo_res.status(), StatusCode::OK);
    let userinfo_body: Value = userinfo_res.json().await.unwrap();
    assert_eq!(userinfo_body["sub"], bob_id);
    assert_eq!(userinfo_body["preferred_username"], "bob_researcher");
    assert_eq!(userinfo_body["email"], "bob@apich.org");

    // ========================================================================
    // TEST 4: Identity Tree & Team Hierarchy Management
    // ========================================================================
    println!("--- Running Test 4: Identity Tree & Teams ---");

    // Login as Platform Admin Alice
    let _ = client
        .post(format!("{}/api/auth/login", base_url))
        .json(&json!({
            "login": "alice_admin",
            "password": "AliceAdmin123!"
        }))
        .send()
        .await
        .unwrap();

    // Create Organization: Quantum Labs
    let org_res = client
        .post(format!("{}/api/orgs", base_url))
        .json(&CreateOrganizationDto {
            name: "Quantum Labs".to_string(),
            slug: "quantum-labs".to_string(),
            description: Some("Advanced Quantum Computing Lab".to_string()),
            ..Default::default()
        })
        .send()
        .await
        .expect("Failed to create org");
    assert_eq!(org_res.status(), StatusCode::OK);
    let org_body: Value = org_res.json().await.unwrap();
    let org_id = uuid::Uuid::parse_str(org_body["id"].as_str().unwrap()).unwrap();

    // Create Root Team: Hardware Engineering
    let root_team_res = client
        .post(format!("{}/api/teams", base_url))
        .json(&CreateTeamDto {
            org_id,
            parent_team_id: None,
            name: "Hardware Engineering".to_string(),
            slug: "hw-eng".to_string(),
            description: Some("Quantum hardware research team".to_string()),
            ..Default::default()
        })
        .send()
        .await
        .expect("Failed to create root team");
    assert_eq!(root_team_res.status(), StatusCode::OK);
    let root_team_body: Value = root_team_res.json().await.unwrap();
    let root_team_id = uuid::Uuid::parse_str(root_team_body["id"].as_str().unwrap()).unwrap();

    // Create Subteam: Cryogenics Subsystem (parent = Hardware Engineering)
    let subteam_res = client
        .post(format!("{}/api/teams", base_url))
        .json(&CreateTeamDto {
            org_id,
            parent_team_id: Some(root_team_id),
            name: "Cryogenics Subsystem".to_string(),
            slug: "cryo-sub".to_string(),
            description: Some("Ultra-low temperature dilution refrigerators".to_string()),
            ..Default::default()
        })
        .send()
        .await
        .expect("Failed to create subteam");
    assert_eq!(subteam_res.status(), StatusCode::OK);

    // Add Bob as Team Admin to Root Team
    let add_member_res = client
        .post(format!("{}/api/teams/{}/members", base_url, root_team_id))
        .json(&json!({
            "user_id": bob_uuid,
            "role": "admin"
        }))
        .send()
        .await
        .expect("Failed to add team member");
    assert_eq!(add_member_res.status(), StatusCode::OK);

    // Query Team Tree
    let tree_res = client
        .get(format!("{}/api/orgs/{}/team-tree", base_url, org_id))
        .send()
        .await
        .expect("Failed to fetch team tree");
    assert_eq!(tree_res.status(), StatusCode::OK);
    let tree_body: Value = tree_res.json().await.unwrap();
    let tree_nodes = tree_body.as_array().unwrap();
    assert_eq!(tree_nodes.len(), 1);
    assert_eq!(tree_nodes[0]["team"]["slug"], "hw-eng");
    assert_eq!(tree_nodes[0]["children"].as_array().unwrap().len(), 1);
    assert_eq!(tree_nodes[0]["children"][0]["team"]["slug"], "cryo-sub");

    // ========================================================================
    // TEST 5: Project Lifecycle, Dedicated Sandbox Container & FastCDC VCS
    // ========================================================================
    println!("--- Running Test 5: Project Sandbox Lifecycle & VCS ---");

    // Create Project under Quantum Labs
    let proj_res = client
        .post(format!("{}/api/projects", base_url))
        .json(&json!({
            "org_id": org_id,
            "team_id": root_team_id,
            "name": "Superconducting Qubits",
            "slug": "sc-qubits",
            "description": "Simulation and control models for transmon qubits"
        }))
        .send()
        .await
        .expect("Failed to create project");

    assert_eq!(proj_res.status(), StatusCode::OK);
    let proj_body: Value = proj_res.json().await.unwrap();
    let proj_id = uuid::Uuid::parse_str(proj_body["id"].as_str().unwrap()).unwrap();
    let storage_path = PathBuf::from(proj_body["storage_path"].as_str().unwrap());

    // Verify VCS initialized in storage path
    assert!(storage_path.exists());
    assert!(storage_path.join(".apich").exists());

    // Launch Dedicated Sandbox Container for (Project + Alice)
    let launch_res = client
        .post(format!("{}/api/projects/{}/sandbox/start", base_url, proj_id))
        .send()
        .await
        .expect("Failed to launch sandbox container");

    let launch_status = launch_res.status();
    let launch_text = launch_res.text().await.unwrap();
    if launch_status != StatusCode::OK {
        eprintln!("LAUNCH SANDBOX FAILED (status {}): {}", launch_status, launch_text);
    }
    assert_eq!(launch_status, StatusCode::OK);
    let launch_body: Value = serde_json::from_str(&launch_text).unwrap();
    assert_eq!(launch_body["status"], "running");
    assert_eq!(launch_body["project_id"], proj_id.to_string());

    // Query Sandbox Status
    let status_res = client
        .get(format!("{}/api/projects/{}/sandbox/status", base_url, proj_id))
        .send()
        .await
        .expect("Failed to get sandbox status");
    assert_eq!(status_res.status(), StatusCode::OK);
    let status_body: Value = status_res.json().await.unwrap();
    assert_eq!(status_body["status"], "running");

    // Stop Sandbox Container
    let stop_res = client
        .post(format!("{}/api/projects/{}/sandbox/stop", base_url, proj_id))
        .send()
        .await
        .expect("Failed to stop sandbox container");
    assert_eq!(stop_res.status(), StatusCode::OK);
    let stop_body: Value = stop_res.json().await.unwrap();
    assert_eq!(stop_body["status"], "stopped");

    // Create a new file in project directory and trigger VCS snapshot
    tokio::fs::write(
        storage_path.join("transmon_hamiltonian.py"),
        b"# Simulation of transmon qubit Hamiltonian\nimport numpy as np\n",
    )
    .await
    .expect("Failed to write test file");

    let snapshot_res = client
        .post(format!("{}/api/projects/{}/vcs/snapshot", base_url, proj_id))
        .json(&json!({
            "message": "Initial calibration equations for transmon simulator"
        }))
        .send()
        .await
        .expect("Failed to snapshot project");
    assert_eq!(snapshot_res.status(), StatusCode::OK);
    let snap_body: Value = snapshot_res.json().await.unwrap();
    assert!(snap_body["message"].as_str().unwrap().starts_with("Initial calibration equations for transmon simulator"));

    // Retrieve VCS Timeline
    let timeline_res = client
        .get(format!("{}/api/projects/{}/vcs/timeline", base_url, proj_id))
        .send()
        .await
        .expect("Failed to get project timeline");
    assert_eq!(timeline_res.status(), StatusCode::OK);
    let timeline_body: Value = timeline_res.json().await.unwrap();
    let snapshots = timeline_body.as_array().unwrap();
    assert!(!snapshots.is_empty());
    assert!(snapshots[0]["message"].as_str().unwrap().starts_with("Initial calibration equations for transmon simulator"));

    // ========================================================================
    // TEST 5.1: Project Sharing, Member Roles & Concurrent Multi-User Sandboxes
    // ========================================================================
    println!("--- Running Test 5.1: Project Sharing & Concurrent Sandboxes ---");

    // Create a separate HTTP client for Bob and log in
    let bob_client = reqwest::Client::builder()
        .cookie_store(true)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Failed to build bob client");

    let bob_login_res = bob_client
        .post(format!("{}/api/auth/login", base_url))
        .json(&json!({
            "login": "bob_researcher",
            "password": "BobSecretPassword123!"
        }))
        .send()
        .await
        .expect("Failed to login bob");
    assert_eq!(bob_login_res.status(), StatusCode::OK);

    // Register David as an external collaborator (viewer)
    let david_client = reqwest::Client::builder()
        .cookie_store(true)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Failed to build david client");

    let david_reg_res = david_client
        .post(format!("{}/api/auth/register", base_url))
        .json(&json!({
            "username": "david_collaborator",
            "email": "david@apich.org",
            "password": "DavidPassword123!",
            "display_name": "David Collaborator"
        }))
        .send()
        .await
        .expect("Failed to register david");
    assert_eq!(david_reg_res.status(), StatusCode::OK);
    let david_body: Value = david_reg_res.json().await.unwrap();
    let david_uuid = uuid::Uuid::parse_str(david_body["user"]["id"].as_str().unwrap()).unwrap();

    // Alice (owner) shares project with Bob as 'editor'
    let add_bob_res = client
        .post(format!("{}/api/projects/{}/members", base_url, proj_id))
        .json(&json!({
            "user_id": bob_uuid,
            "role": "editor"
        }))
        .send()
        .await
        .expect("Failed to add Bob as editor");
    assert_eq!(add_bob_res.status(), StatusCode::OK);

    // Alice shares project with David as 'viewer'
    let add_david_res = client
        .post(format!("{}/api/projects/{}/members", base_url, proj_id))
        .json(&json!({
            "user_id": david_uuid,
            "role": "viewer"
        }))
        .send()
        .await
        .expect("Failed to add David as viewer");
    assert_eq!(add_david_res.status(), StatusCode::OK);

    // Alice lists project members: should contain Alice (owner), Bob (editor), David (viewer)
    let members_res = client
        .get(format!("{}/api/projects/{}/members", base_url, proj_id))
        .send()
        .await
        .expect("Failed to list members");
    assert_eq!(members_res.status(), StatusCode::OK);
    let members_list: Value = members_res.json().await.unwrap();
    let members = members_list.as_array().unwrap();
    assert_eq!(members.len(), 3);

    // Verify Bob sees shared project in his dashboard project list
    let bob_projects_res = bob_client
        .get(format!("{}/api/projects", base_url))
        .send()
        .await
        .expect("Failed to list bob projects");
    assert_eq!(bob_projects_res.status(), StatusCode::OK);
    let bob_projects: Value = bob_projects_res.json().await.unwrap();
    assert!(bob_projects.as_array().unwrap().iter().any(|p| p["id"] == proj_id.to_string()));

    // Verify David (Viewer) read access vs write restrictions
    let david_view_res = david_client
        .get(format!("{}/api/projects/{}", base_url, proj_id))
        .send()
        .await
        .expect("Failed to fetch project as viewer");
    assert_eq!(david_view_res.status(), StatusCode::OK);

    let david_timeline_res = david_client
        .get(format!("{}/api/projects/{}/vcs/timeline", base_url, proj_id))
        .send()
        .await
        .expect("Failed to get timeline as viewer");
    assert_eq!(david_timeline_res.status(), StatusCode::OK);

    // David (viewer) attempts to launch sandbox -> 403 Forbidden
    let david_sandbox_start = david_client
        .post(format!("{}/api/projects/{}/sandbox/start", base_url, proj_id))
        .send()
        .await
        .expect("Failed to send start sandbox");
    assert_eq!(david_sandbox_start.status(), StatusCode::FORBIDDEN);

    // David (viewer) attempts to create snapshot -> 403 Forbidden
    let david_snap = david_client
        .post(format!("{}/api/projects/{}/vcs/snapshot", base_url, proj_id))
        .json(&json!({ "message": "Viewer snapshot attempt" }))
        .send()
        .await
        .expect("Failed to send snapshot");
    assert_eq!(david_snap.status(), StatusCode::FORBIDDEN);

    // Concurrent Sandboxes: Alice and Bob launch sandboxes on the SAME project simultaneously
    let alice_start = client
        .post(format!("{}/api/projects/{}/sandbox/start", base_url, proj_id))
        .send()
        .await
        .expect("Failed to start Alice sandbox");
    assert_eq!(alice_start.status(), StatusCode::OK);
    let alice_box: Value = alice_start.json().await.unwrap();
    assert_eq!(alice_box["status"], "running");

    let bob_start = bob_client
        .post(format!("{}/api/projects/{}/sandbox/start", base_url, proj_id))
        .send()
        .await
        .expect("Failed to start Bob sandbox");
    assert_eq!(bob_start.status(), StatusCode::OK);
    let bob_box: Value = bob_start.json().await.unwrap();
    assert_eq!(bob_box["status"], "running");

    // Both containers are distinct and isolated
    assert_ne!(alice_box["container_name"], bob_box["container_name"]);

    // Both Alice and Bob query their own sandbox status -> both running
    let alice_status = client
        .get(format!("{}/api/projects/{}/sandbox/status", base_url, proj_id))
        .send()
        .await
        .expect("Failed to get Alice status");
    let alice_status_body: Value = alice_status.json().await.unwrap();
    assert_eq!(alice_status_body["status"], "running");

    let bob_status = bob_client
        .get(format!("{}/api/projects/{}/sandbox/status", base_url, proj_id))
        .send()
        .await
        .expect("Failed to get Bob status");
    let bob_status_body: Value = bob_status.json().await.unwrap();
    assert_eq!(bob_status_body["status"], "running");

    // Alice stops her sandbox -> Bob's sandbox continues running independently
    let alice_stop = client
        .post(format!("{}/api/projects/{}/sandbox/stop", base_url, proj_id))
        .send()
        .await
        .expect("Failed to stop Alice sandbox");
    assert_eq!(alice_stop.status(), StatusCode::OK);

    let alice_status_after = client
        .get(format!("{}/api/projects/{}/sandbox/status", base_url, proj_id))
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap();
    assert_eq!(alice_status_after["status"], "stopped");

    let bob_status_after = bob_client
        .get(format!("{}/api/projects/{}/sandbox/status", base_url, proj_id))
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap();
    assert_eq!(bob_status_after["status"], "running");

    // Now Bob stops his sandbox
    let bob_stop = bob_client
        .post(format!("{}/api/projects/{}/sandbox/stop", base_url, proj_id))
        .send()
        .await
        .expect("Failed to stop Bob sandbox");
    assert_eq!(bob_stop.status(), StatusCode::OK);

    // Alice updates David's role to 'editor'
    let update_role_res = client
        .put(format!("{}/api/projects/{}/members/{}", base_url, proj_id, david_uuid))
        .json(&json!({ "role": "editor" }))
        .send()
        .await
        .expect("Failed to update role");
    assert_eq!(update_role_res.status(), StatusCode::OK);

    // Alice removes David from project members
    let remove_member_res = client
        .delete(format!("{}/api/projects/{}/members/{}", base_url, proj_id, david_uuid))
        .send()
        .await
        .expect("Failed to remove member");
    assert_eq!(remove_member_res.status(), StatusCode::OK);

    let members_after_removal: Value = client
        .get(format!("{}/api/projects/{}/members", base_url, proj_id))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(members_after_removal.as_array().unwrap().len(), 2);

    // ========================================================================
    // TEST 6: Admin Registration Mode & Invitation Workflows
    // ========================================================================
    println!("--- Running Test 6: Registration Mode & Invitations ---");

    // Admin updates registration mode to "invite_only"
    let update_mode_res = client
        .put(format!("{}/api/admin/settings", base_url))
        .json(&json!({
            "registration_mode": "invite_only"
        }))
        .send()
        .await
        .expect("Failed to update system settings");
    assert_eq!(update_mode_res.status(), StatusCode::OK);

    // Attempt anonymous registration without invite token -> 403 Forbidden
    let blocked_res = client
        .post(format!("{}/api/auth/register", base_url))
        .json(&json!({
            "username": "intruder_user",
            "email": "intruder@external.com",
            "password": "Password123!",
            "display_name": "Intruder"
        }))
        .send()
        .await
        .expect("Failed to send register request");
    assert_eq!(blocked_res.status(), StatusCode::FORBIDDEN);

    // Admin issues an invitation for Charlie
    let invite_res = client
        .post(format!("{}/api/admin/invitations", base_url))
        .json(&json!({
            "email": "charlie@apich.org",
            "org_id": org_id,
            "team_id": root_team_id,
            "role": "member"
        }))
        .send()
        .await
        .expect("Failed to create invitation");
    assert_eq!(invite_res.status(), StatusCode::OK);
    let invite_body: Value = invite_res.json().await.unwrap();
    let invite_token = invite_body["token"].as_str().unwrap().to_string();

    // Verify invitation email was recorded in MailerService
    let sent_emails = state.mailer.get_sent_emails();
    assert!(!sent_emails.is_empty());
    let email = sent_emails.iter().find(|e| e.to == "charlie@apich.org").unwrap();
    assert!(email.body.contains(&invite_token));

    // Charlie completes registration with invite token -> 200 OK
    let charlie_reg_res = client
        .post(format!("{}/api/auth/register", base_url))
        .json(&json!({
            "username": "charlie_researcher",
            "email": "charlie@apich.org",
            "password": "CharliePassword123!",
            "display_name": "Charlie Postdoc",
            "invite_token": invite_token
        }))
        .send()
        .await
        .expect("Failed to register with invitation");
    assert_eq!(charlie_reg_res.status(), StatusCode::OK);
    let charlie_body: Value = charlie_reg_res.json().await.unwrap();
    let charlie_uuid = uuid::Uuid::parse_str(charlie_body["user"]["id"].as_str().unwrap()).unwrap();

    // Reusing the same invite token should now fail
    let reused_res = client
        .post(format!("{}/api/auth/register", base_url))
        .json(&json!({
            "username": "charlie_clone",
            "email": "charlie@apich.org",
            "password": "Password123!",
            "display_name": "Charlie Clone",
            "invite_token": invite_token
        }))
        .send()
        .await
        .expect("Failed to test reuse token");
    assert_ne!(reused_res.status(), StatusCode::OK);

    // ========================================================================
    // TEST 7: Leptos SSR Frontend Page Renders, i18n & Admin Management Flows
    // ========================================================================
    println!("--- Running Test 7: Leptos SSR HTML Page Rendering, i18n & Admin Management ---");

    // Login as Alice (owner of proj_id and platform admin)
    let _ = client
        .post(format!("{}/api/auth/login", base_url))
        .json(&json!({
            "login": "alice_admin",
            "password": "AliceAdmin123!"
        }))
        .send()
        .await
        .unwrap();

    // 7.1 Primary Language Default (English) Dashboard
    let home_res = client.get(format!("{}/", base_url)).send().await.unwrap();
    assert_eq!(home_res.status(), StatusCode::OK);
    let home_html = home_res.text().await.unwrap();
    assert!(home_html.contains("APICH"));
    // Leptos SSR correctly HTML-escapes text nodes ("&" -> "&amp;"), unlike the old raw format! templates.
    assert!(home_html.contains("Projects &amp; Sandboxes") || home_html.contains("Projects & Sandboxes"));
    assert!(home_html.contains("New Project"));

    // 7.2 Language Toggle: Switch to Chinese (zh)
    let lang_zh_res = client.get(format!("{}/set-lang?lang=zh&return_to=/", base_url)).send().await.unwrap();
    assert_eq!(lang_zh_res.status(), StatusCode::SEE_OTHER);
    assert_eq!(lang_zh_res.headers().get("location").unwrap(), "/");

    let zh_home_res = client.get(format!("{}/", base_url)).send().await.unwrap();
    assert_eq!(zh_home_res.status(), StatusCode::OK);
    let zh_home_html = zh_home_res.text().await.unwrap();
    assert!(zh_home_html.contains("科研项目与计算沙箱"));
    assert!(zh_home_html.contains("新建项目"));

    // Switch back to English (primary)
    let _ = client.get(format!("{}/set-lang?lang=en&return_to=/", base_url)).send().await.unwrap();

    // Authenticated user accessing /login or /register is redirected to /
    let auth_login_res = client.get(format!("{}/login", base_url)).send().await.unwrap();
    assert_eq!(auth_login_res.status(), StatusCode::SEE_OTHER);
    assert_eq!(auth_login_res.headers().get("location").unwrap(), "/");

    let auth_reg_res = client.get(format!("{}/register", base_url)).send().await.unwrap();
    assert_eq!(auth_reg_res.status(), StatusCode::SEE_OTHER);
    assert_eq!(auth_reg_res.headers().get("location").unwrap(), "/");

    // Settings page (Profile chip badge, no fake inputs)
    let settings_res = client.get(format!("{}/settings", base_url)).send().await.unwrap();
    assert_eq!(settings_res.status(), StatusCode::OK);
    let settings_html = settings_res.text().await.unwrap();
    assert!(settings_html.contains("Account &amp; Security Settings") || settings_html.contains("Account & Security Settings"));
    assert!(settings_html.contains("Platform Administrator"));
    assert!(settings_html.contains("Passkey"));

    // 7.2.1 Personal Profile Deep Modification via Web Form
    let update_prof_res = client
        .post(format!("{}/settings/profile", base_url))
        .form(&[
            ("display_name", "Alice Principal Investigator"),
            ("email", "alice.chief@apich.org"),
            ("avatar_url", "https://apich.org/avatars/alice.png"),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(update_prof_res.status(), StatusCode::SEE_OTHER);
    assert!(update_prof_res.headers().get("location").unwrap().to_str().unwrap().contains("notice="));

    let alice_db = repo.get_user_by_id(alice.id).await.unwrap().unwrap();
    assert_eq!(alice_db.display_name, "Alice Principal Investigator");
    assert_eq!(alice_db.email, "alice.chief@apich.org");
    assert_eq!(alice_db.avatar_url.as_deref(), Some("https://apich.org/avatars/alice.png"));

    // 7.2.2 Password Change Validations & Re-login
    // Case A: Password confirmation mismatch
    let pwd_mismatch_res = client
        .post(format!("{}/settings/password", base_url))
        .form(&[
            ("current_password", "AliceAdmin123!"),
            ("new_password", "NewSecurePassword999!"),
            ("confirm_password", "MismatchPassword999!"),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(pwd_mismatch_res.status(), StatusCode::SEE_OTHER);
    assert!(pwd_mismatch_res.headers().get("location").unwrap().to_str().unwrap().contains("error="));

    // Case B: Incorrect old password
    let pwd_wrong_res = client
        .post(format!("{}/settings/password", base_url))
        .form(&[
            ("current_password", "WrongOldPassword!"),
            ("new_password", "NewSecurePassword999!"),
            ("confirm_password", "NewSecurePassword999!"),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(pwd_wrong_res.status(), StatusCode::SEE_OTHER);
    assert!(pwd_wrong_res.headers().get("location").unwrap().to_str().unwrap().contains("error="));

    // Case C: Valid password change
    let pwd_ok_res = client
        .post(format!("{}/settings/password", base_url))
        .form(&[
            ("current_password", "AliceAdmin123!"),
            ("new_password", "NewSecurePassword999!"),
            ("confirm_password", "NewSecurePassword999!"),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(pwd_ok_res.status(), StatusCode::SEE_OTHER);
    assert!(pwd_ok_res.headers().get("location").unwrap().to_str().unwrap().contains("notice="));

    // Verify Alice can authenticate with the new password
    let new_login_res = client
        .post(format!("{}/api/auth/login", base_url))
        .json(&json!({
            "login": "alice_admin",
            "password": "NewSecurePassword999!"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(new_login_res.status(), StatusCode::OK);

    // 7.3 Organization & Team Admin Management
    let orgs_res = client.get(format!("{}/admin/orgs", base_url)).send().await.unwrap();
    assert_eq!(orgs_res.status(), StatusCode::OK);
    let orgs_html = orgs_res.text().await.unwrap();
    assert!(orgs_html.contains("Organizations &amp; Teams") || orgs_html.contains("Organizations & Teams"));
    assert!(orgs_html.contains("Quantum Labs"));
    assert!(orgs_html.contains("Hardware Engineering"));

    // 7.3.1 Organization Deep Modification & Deletion via Web Forms
    let create_org_res = client
        .post(format!("{}/admin/orgs/new", base_url))
        .form(&[
            ("name", "CERN Quantum Physics"),
            ("slug", "cern-quantum"),
            ("description", "High Energy Physics Institute"),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(create_org_res.status(), StatusCode::SEE_OTHER);
    assert!(create_org_res.headers().get("location").unwrap().to_str().unwrap().contains("org_created"));

    let cern_org = repo.get_organization_by_slug("cern-quantum").await.unwrap().unwrap();
    assert_eq!(cern_org.name, "CERN Quantum Physics");

    let edit_org_res = client
        .post(format!("{}/admin/orgs/edit", base_url))
        .form(&[
            ("org_id", cern_org.id.to_string()),
            ("name", "CERN Advanced Quantum Institute".to_string()),
            ("slug", "cern-quantum-adv".to_string()),
            ("description", "Updated description for CERN".to_string()),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(edit_org_res.status(), StatusCode::SEE_OTHER);
    assert!(edit_org_res.headers().get("location").unwrap().to_str().unwrap().contains("org_updated"));

    let updated_cern_org = repo.get_organization_by_id(cern_org.id).await.unwrap().unwrap();
    assert_eq!(updated_cern_org.name, "CERN Advanced Quantum Institute");
    assert_eq!(updated_cern_org.slug, "cern-quantum-adv");

    let del_org_res = client
        .post(format!("{}/admin/orgs/delete", base_url))
        .form(&[
            ("org_id", cern_org.id.to_string()),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(del_org_res.status(), StatusCode::SEE_OTHER);
    assert!(del_org_res.headers().get("location").unwrap().to_str().unwrap().contains("org_deleted"));
    assert!(repo.get_organization_by_id(cern_org.id).await.unwrap().is_none());

    // 7.3.2 Team Creation, Hierarchy Re-parenting & Deletion via Web Forms
    let create_team_res = client
        .post(format!("{}/admin/teams/new", base_url))
        .form(&[
            ("org_id", org_id.to_string()),
            ("name", "Quantum Theory".to_string()),
            ("slug", "quantum-theory".to_string()),
            ("description", "Theoretical physics subteam".to_string()),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(create_team_res.status(), StatusCode::SEE_OTHER);
    assert!(create_team_res.headers().get("location").unwrap().to_str().unwrap().contains("team_created"));

    let org_teams = repo.list_teams_by_org(org_id).await.unwrap();
    let qt_team = org_teams.into_iter().find(|t| t.slug == "quantum-theory").unwrap();
    assert_eq!(qt_team.name, "Quantum Theory");

    // Edit team and re-parent under root_team_id
    let edit_team_res = client
        .post(format!("{}/admin/teams/edit", base_url))
        .form(&[
            ("team_id", qt_team.id.to_string()),
            ("name", "Quantum Theoretical Simulation".to_string()),
            ("slug", "quantum-theory-sim".to_string()),
            ("description", "Re-parented sub-theory group".to_string()),
            ("parent_team_id", root_team_id.to_string()),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(edit_team_res.status(), StatusCode::SEE_OTHER);
    assert!(edit_team_res.headers().get("location").unwrap().to_str().unwrap().contains("team_updated"));

    let updated_qt_team = repo.get_team_by_id(qt_team.id).await.unwrap().unwrap();
    assert_eq!(updated_qt_team.name, "Quantum Theoretical Simulation");
    assert_eq!(updated_qt_team.parent_team_id, Some(root_team_id));

    // Admin adds team member via web form
    let add_tm_res = client
        .post(format!("{}/admin/teams/members/add", base_url))
        .form(&[
            ("team_id", root_team_id.to_string()),
            ("user_id_or_username", "bob_researcher".to_string()),
            ("role", "lead".to_string()),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(add_tm_res.status(), StatusCode::SEE_OTHER);

    // Admin removes team member via web form
    let rm_tm_res = client
        .post(format!("{}/admin/teams/members/remove", base_url))
        .form(&[
            ("team_id", root_team_id.to_string()),
            ("user_id", bob_uuid.to_string()),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(rm_tm_res.status(), StatusCode::SEE_OTHER);

    // Delete team via web form
    let del_team_res = client
        .post(format!("{}/admin/teams/delete", base_url))
        .form(&[
            ("team_id", qt_team.id.to_string()),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(del_team_res.status(), StatusCode::SEE_OTHER);
    assert!(del_team_res.headers().get("location").unwrap().to_str().unwrap().contains("team_deleted"));
    assert!(repo.get_team_by_id(qt_team.id).await.unwrap().is_none());

    // 7.3.3 Outbound Email (SMTP) Platform Settings & Verification Test Form
    let platform_res = client.get(format!("{}/admin/platform", base_url)).send().await.unwrap();
    assert_eq!(platform_res.status(), StatusCode::OK);
    let platform_html = platform_res.text().await.unwrap();
    assert!(platform_html.contains("System Admin"));
    assert!(platform_html.contains("SMTP Outbound Mail Server"));
    assert!(platform_html.contains("Send Test Email"));

    // Update full SMTP settings via web form
    let update_smtp_res = client
        .post(format!("{}/admin/platform/settings", base_url))
        .form(&[
            ("section", "smtp"),
            ("smtp_enabled", "true"),
            ("smtp_host", ""),
            ("smtp_port", "587"),
            ("smtp_username", "apich_admin"),
            ("smtp_password", "SmtpSuperSecretPassword!"),
            ("smtp_from_email", "no-reply@apich.edu"),
            ("smtp_from_name", "APICH Academic Platform"),
            ("smtp_use_tls", "true"),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(update_smtp_res.status(), StatusCode::SEE_OTHER);
    assert!(update_smtp_res.headers().get("location").unwrap().to_str().unwrap().contains("notice="));

    let settings = repo.get_system_settings().await.unwrap();
    assert!(settings.smtp_enabled);
    assert_eq!(settings.smtp_port, Some(587));
    assert_eq!(settings.smtp_from_email.as_deref(), Some("no-reply@apich.edu"));
    assert_eq!(settings.smtp_from_name.as_deref(), Some("APICH Academic Platform"));
    assert!(settings.smtp_use_tls);

    // Trigger live test verification email via web form
    let test_smtp_res = client
        .post(format!("{}/admin/platform/smtp-test", base_url))
        .form(&[
            ("test_email", "researcher-verify@apich.edu"),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(test_smtp_res.status(), StatusCode::SEE_OTHER);
    assert!(test_smtp_res.headers().get("location").unwrap().to_str().unwrap().contains("notice="));

    // Verify in mailer audit queue that test verification email was recorded
    let sent_emails = state.mailer.get_sent_emails();
    assert!(sent_emails.iter().any(|m| m.to == "researcher-verify@apich.edu" && m.subject.contains("SMTP Delivery Test")));

    // 7.4 Project Detail Page Tabs & Collaborator Management
    // Bare /projects/:id now defaults to the Files tab (the old "Overview" tab was dropped:
    // its content was redundant with the page header, and plan.md's spec is Files/VCS/Sharing).
    let proj_files_res = client.get(format!("{}/projects/{}", base_url, proj_id)).send().await.unwrap();
    assert_eq!(proj_files_res.status(), StatusCode::OK);
    let proj_files_html = proj_files_res.text().await.unwrap();
    assert!(proj_files_html.contains("Project Files"));
    assert!(proj_files_html.contains("Files"));

    // merge/timeline/git are now unified into a single VCS tab (plan.md: "VCS history & management").
    let proj_vcs_res = client.get(format!("{}/projects/{}?tab=vcs", base_url, proj_id)).send().await.unwrap();
    assert_eq!(proj_vcs_res.status(), StatusCode::OK);
    let proj_vcs_html = proj_vcs_res.text().await.unwrap();
    assert!(proj_vcs_html.contains("Branches &amp; Merging") || proj_vcs_html.contains("Branches & Merging"));
    assert!(proj_vcs_html.contains("Timeline Snapshots"));
    assert!(proj_vcs_html.contains("Initial calibration equations for transmon simulator"));
    assert!(proj_vcs_html.contains("Git Compatibility"));

    // Legacy tab query values still resolve to the same VCS tab.
    let proj_merge_res = client.get(format!("{}/projects/{}?tab=merge", base_url, proj_id)).send().await.unwrap();
    assert_eq!(proj_merge_res.status(), StatusCode::OK);
    let proj_merge_html = proj_merge_res.text().await.unwrap();
    assert!(proj_merge_html.contains("Branches &amp; Merging") || proj_merge_html.contains("Branches & Merging"));

    // Owner adds collaborator via web form
    let add_collab_res = client
        .post(format!("{}/projects/{}/members/add", base_url, proj_id))
        .form(&[
            ("user_id_or_username", "charlie_researcher".to_string()),
            ("role", "editor".to_string()),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(add_collab_res.status(), StatusCode::SEE_OTHER);
    assert!(add_collab_res.headers().get("location").unwrap().to_str().unwrap().contains("collaborator_added"));

    // Owner removes collaborator via web form
    let rm_collab_res = client
        .post(format!("{}/projects/{}/members/remove", base_url, proj_id))
        .form(&[
            ("user_id", charlie_uuid.to_string()),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(rm_collab_res.status(), StatusCode::SEE_OTHER);
    assert!(rm_collab_res.headers().get("location").unwrap().to_str().unwrap().contains("collaborator_removed"));

    // ========================================================================
    // TEST 7.5: Scientific Workspace Extensions: Hub Links, SQLite Tables, Knowledge Hub, Terminal
    // ========================================================================
    println!("--- Running Test 7.5: Hub Links, SQLite Tables, Knowledge Hub & Terminal ---");

    // A. External Hub Links Resolution, Team-level Overrides and Org Lockout
    let _ = repo.update_organization(org_id, UpdateOrganizationDto {
        name: None,
        slug: None,
        description: None,
        chat_url: Some("https://matrix.quantum-labs.org".to_string()),
        meeting_url: Some("https://meet.quantum-labs.org".to_string()),
        drive_url: Some("https://drive.quantum-labs.org".to_string()),
        ai_agent_url: Some("https://ai.quantum-labs.org".to_string()),
        allow_team_override: Some(true),
    }).await.unwrap();

    let _ = repo.update_team(root_team_id, UpdateTeamDto {
        name: None,
        slug: None,
        description: None,
        parent_team_id: None,
        chat_url: Some("https://discord.gg/quantum-hw".to_string()),
        meeting_url: None, // Inherits org
        drive_url: None,   // Inherits org
        ai_agent_url: None, // Inherits org
    }).await.unwrap();

    // Query effective hub links: team override on chat, inherited org on meeting/drive/ai
    let hl_res = client.get(format!("{}/api/projects/{}/hub-links", base_url, proj_id)).send().await.unwrap();
    assert_eq!(hl_res.status(), StatusCode::OK);
    let hl: Value = hl_res.json().await.unwrap();
    assert_eq!(hl["chat_url"], "https://discord.gg/quantum-hw");
    assert_eq!(hl["meeting_url"], "https://meet.quantum-labs.org");
    assert_eq!(hl["drive_url"], "https://drive.quantum-labs.org");
    assert_eq!(hl["ai_agent_url"], "https://ai.quantum-labs.org");

    // Master Lockout: Org admin disables allow_team_override
    let _ = repo.update_organization(org_id, UpdateOrganizationDto {
        name: None,
        slug: None,
        description: None,
        chat_url: None,
        meeting_url: None,
        drive_url: None,
        ai_agent_url: None,
        allow_team_override: Some(false),
    }).await.unwrap();

    let hl_lockout_res = client.get(format!("{}/api/projects/{}/hub-links", base_url, proj_id)).send().await.unwrap();
    assert_eq!(hl_lockout_res.status(), StatusCode::OK);
    let hl_lockout: Value = hl_lockout_res.json().await.unwrap();
    assert_eq!(hl_lockout["chat_url"], "https://matrix.quantum-labs.org");

    // B. SQLite Tables Engine: Database creation, SQL DDL/DML, Grid and Console views
    let create_db_res = client
        .post(format!("{}/projects/{}/table/create-db", base_url, proj_id))
        .send()
        .await
        .unwrap();
    assert_eq!(create_db_res.status(), StatusCode::SEE_OTHER);
    assert!(storage_path.join("data.db").exists());

    // Execute DDL and DML via API
    let sql_exec_res = client
        .post(format!("{}/api/projects/{}/sql/execute", base_url, proj_id))
        .json(&json!({
            "file": "data.db",
            "sql": "CREATE TABLE qubit_telemetry (id INTEGER PRIMARY KEY, qid TEXT, t1_us REAL, t2_us REAL); INSERT INTO qubit_telemetry VALUES (1, 'Transmon_Q0', 88.4, 65.1), (2, 'Transmon_Q1', 94.7, 72.3);"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(sql_exec_res.status(), StatusCode::OK);
    let sql_body: Value = sql_exec_res.json().await.unwrap();
    assert!(sql_body["message"].as_str().unwrap().contains("executed successfully"));
    assert_eq!(sql_body["is_query"], false);

    // Also test a SELECT query via API
    let select_exec_res = client
        .post(format!("{}/api/projects/{}/sql/execute", base_url, proj_id))
        .json(&json!({
            "file": "data.db",
            "sql": "SELECT qid, t1_us FROM qubit_telemetry WHERE t1_us > 90.0;"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(select_exec_res.status(), StatusCode::OK);
    let select_body: Value = select_exec_res.json().await.unwrap();
    assert_eq!(select_body["is_query"], true);
    assert!(select_body["message"].as_str().unwrap().contains("Query executed successfully"));
    assert_eq!(select_body["rows"].as_array().unwrap().len(), 1);

    // Fetch Table Data via API
    let table_data_res = client
        .get(format!("{}/api/projects/{}/tables/data?file=data.db&table=qubit_telemetry", base_url, proj_id))
        .send()
        .await
        .unwrap();
    assert_eq!(table_data_res.status(), StatusCode::OK);
    let td_body: Value = table_data_res.json().await.unwrap();
    assert_eq!(td_body["total_rows"], 2);
    assert_eq!(td_body["columns"][1], "qid");
    assert_eq!(td_body["rows"][0][1], "Transmon_Q0");

    // Web UI: Visual Grid View
    let grid_page_res = client
        .get(format!("{}/projects/{}/table?file=data.db&table=qubit_telemetry&mode=grid", base_url, proj_id))
        .send()
        .await
        .unwrap();
    assert_eq!(grid_page_res.status(), StatusCode::OK);
    let grid_html = grid_page_res.text().await.unwrap();
    assert!(grid_html.contains("qubit_telemetry"));
    assert!(grid_html.contains("Transmon_Q0"));
    assert!(grid_html.contains("spreadsheet-grid") || grid_html.contains("Visual Grid"));

    // Web UI: Execute SQL in Console Form
    let console_sql_res = client
        .post(format!("{}/projects/{}/table/sql", base_url, proj_id))
        .form(&[
            ("file", "data.db"),
            ("sql", "SELECT AVG(t1_us) as avg_t1 FROM qubit_telemetry;"),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(console_sql_res.status(), StatusCode::OK);
    let console_html = console_sql_res.text().await.unwrap();
    assert!(console_html.contains("avg_t1"));
    assert!(console_html.contains("91.55"));

    // C. Knowledge Hub: Markdown Tasks, Kanban, Wiki Graph, Calendar, and Bidirectional Sync
    let lab_notes_content = "# Lab Notebook: Quantum Transmon Benchmarking\nSee [[Dilution Fridge Guide]] and [[Pulse Calibration]] for operations.\n\n## Action Items\n- [ ] Characterize resonator frequency response #hardware @2026-09-25\n- [/] Calibrate single-qubit Clifford gates #control @2026-09-28\n- [x] Room-temperature microwave line testing #rf @2026-09-12\n";
    tokio::fs::write(storage_path.join("lab_notebook.md"), lab_notes_content).await.unwrap();

    // Query Knowledge Tasks API
    let tasks_res = client
        .get(format!("{}/api/projects/{}/knowledge/tasks", base_url, proj_id))
        .send()
        .await
        .unwrap();
    assert_eq!(tasks_res.status(), StatusCode::OK);
    let tasks_body: Value = tasks_res.json().await.unwrap();
    assert_eq!(tasks_body.as_array().unwrap().len(), 3);

    // Query Knowledge Kanban API
    let kanban_res = client
        .get(format!("{}/api/projects/{}/knowledge/kanban", base_url, proj_id))
        .send()
        .await
        .unwrap();
    assert_eq!(kanban_res.status(), StatusCode::OK);
    let kanban_body: Value = kanban_res.json().await.unwrap();
    assert_eq!(kanban_body["total_tasks"], 3);
    assert_eq!(kanban_body["completed_tasks"], 1);

    // Query Knowledge Wiki Graph API
    let graph_res = client
        .get(format!("{}/api/projects/{}/knowledge/graph", base_url, proj_id))
        .send()
        .await
        .unwrap();
    assert_eq!(graph_res.status(), StatusCode::OK);
    let graph_body: Value = graph_res.json().await.unwrap();
    let nodes = graph_body["nodes"].as_array().unwrap();
    assert!(nodes.iter().any(|n| n["label"] == "Dilution Fridge Guide"));
    assert!(nodes.iter().any(|n| n["label"] == "Pulse Calibration"));

    // Query Calendar API
    let cal_res = client
        .get(format!("{}/api/projects/{}/knowledge/calendar", base_url, proj_id))
        .send()
        .await
        .unwrap();
    assert_eq!(cal_res.status(), StatusCode::OK);
    let cal_body: Value = cal_res.json().await.unwrap();
    assert_eq!(cal_body.as_array().unwrap().len(), 3);

    // Bidirectional Task Toggle: Toggle line 5 from todo to done
    let toggle_res = client
        .post(format!("{}/projects/{}/knowledge/toggle-task", base_url, proj_id))
        .form(&[
            ("file", "lab_notebook.md"),
            ("line_number", "5"),
            ("status", "done"),
            ("view", "kanban"),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(toggle_res.status(), StatusCode::SEE_OTHER);

    // Verify physical Markdown file was updated!
    let updated_notes = tokio::fs::read_to_string(storage_path.join("lab_notebook.md")).await.unwrap();
    assert!(updated_notes.contains("- [x] Characterize resonator frequency response"));

    // Verify Kanban/Wiki/Calendar Views -- these used to be a standalone `/knowledge` page that
    // nothing in the UI actually linked to (a real orphaned-page bug). plan.md calls for one
    // integrated space ("一体化空间") aggregating notes, wiki, whiteboard, calendar, and kanban,
    // so they're tabs on the unified note page now.
    let kb_page_res = client.get(format!("{}/projects/{}/note?view=kanban", base_url, proj_id)).send().await.unwrap();
    assert_eq!(kb_page_res.status(), StatusCode::OK);
    let kb_page_html = kb_page_res.text().await.unwrap();
    assert!(kb_page_html.contains("kanban-grid"));
    assert!(kb_page_html.contains("📋 Kanban"));

    let wiki_page_res = client.get(format!("{}/projects/{}/note?view=wiki", base_url, proj_id)).send().await.unwrap();
    assert_eq!(wiki_page_res.status(), StatusCode::OK);
    let wiki_page_html = wiki_page_res.text().await.unwrap();
    assert!(wiki_page_html.contains("Notes &amp; Concepts") || wiki_page_html.contains("Notes & Concepts"));

    let cal_page_res = client.get(format!("{}/projects/{}/note?view=calendar", base_url, proj_id)).send().await.unwrap();
    assert_eq!(cal_page_res.status(), StatusCode::OK);

    // The old `/knowledge` page route survives only to redirect bookmarked links to the right
    // place on the unified note page (this test client has redirect-following disabled, like
    // every other request in this test, so check the Location header directly).
    let legacy_kb_res = client.get(format!("{}/projects/{}/knowledge?view=kanban", base_url, proj_id)).send().await.unwrap();
    assert_eq!(legacy_kb_res.status(), StatusCode::SEE_OTHER);
    assert!(legacy_kb_res.headers().get("location").unwrap().to_str().unwrap().contains("/note?view=kanban"));

    // D. Interactive Container Terminal Page & Execution
    let term_page_res = client.get(format!("{}/projects/{}/terminal", base_url, proj_id)).send().await.unwrap();
    assert_eq!(term_page_res.status(), StatusCode::OK);
    let term_html = term_page_res.text().await.unwrap();
    assert!(term_html.contains("Terminal"));
    // The old page claimed specific AI agent tools (aider/goose/antigravity) were
    // pre-installed in the sandbox image with no evidence anywhere that they actually are --
    // dropped as an unverifiable claim plan.md explicitly asks not to make.
    // The terminal itself is now a real Leptos island (apich_islands::TerminalIsland), not a
    // hand-written JS `<input id="term-input">` -- verify the real island marker instead.
    assert!(term_html.contains("leptos-island"));
    assert!(term_html.contains("data-component=\"TerminalIsland_"));

    let term_exec_res = client
        .post(format!("{}/projects/{}/terminal/exec", base_url, proj_id))
        .form(&[
            ("command", "echo 'APICH Sandbox Terminal Live Test'"),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(term_exec_res.status(), StatusCode::OK);
    let exec_res_body: Value = term_exec_res.json().await.unwrap();
    assert!(exec_res_body["output"].as_str().unwrap().contains("APICH Sandbox Terminal Live Test"));

    // 7.6 Anonymous / Unauthenticated Client Flows
    let anon_client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    // Default visit to / when unauthenticated redirects to /login
    let unauth_home_res = anon_client.get(format!("{}/", base_url)).send().await.unwrap();
    assert_eq!(unauth_home_res.status(), StatusCode::SEE_OTHER);
    assert_eq!(unauth_home_res.headers().get("location").unwrap(), "/login");

    // Login page renders in English by default
    let login_res = anon_client.get(format!("{}/login", base_url)).send().await.unwrap();
    assert_eq!(login_res.status(), StatusCode::OK);
    let login_html = login_res.text().await.unwrap();
    assert!(login_html.contains("Sign In to Workspace"));
    assert!(login_html.contains("Passkey"));
    assert!(login_html.contains("Username or Email"));

    // Register page renders in English by default
    let register_res = anon_client.get(format!("{}/register", base_url)).send().await.unwrap();
    assert_eq!(register_res.status(), StatusCode::OK);
    let register_html = register_res.text().await.unwrap();
    assert!(register_html.contains("Create Research Account"));
    assert!(register_html.contains("create_org"));

    // ========================================================================
    // TEST 7.7: Redesigned Showcase Demo, 3-Tier Settings, File Browser, Studios, and Spreadsheets
    // ========================================================================
    println!("--- Running Test 7.7: Redesigned Showcase Demo, 3-Tier Settings, File Browser, Studios & Spreadsheets ---");

    // 1. Verify 3-Tier Admin Settings on Sidebar Shell for Alice (Platform Admin)
    let dash_res = client.get(format!("{}/", base_url)).send().await.unwrap();
    assert_eq!(dash_res.status(), StatusCode::OK);
    let dash_html = dash_res.text().await.unwrap();
    assert!(dash_html.contains("/admin/platform"));
    assert!(dash_html.contains("/admin/orgs"));
    assert!(dash_html.contains("/settings"));

    // 2. Create Showcase Demo Project via POST /projects/demo/create
    let create_demo_res = client.post(format!("{}/projects/demo/create", base_url)).send().await.unwrap();
    assert_eq!(create_demo_res.status(), StatusCode::SEE_OTHER);
    let demo_proj_loc = create_demo_res.headers().get("location").unwrap().to_str().unwrap();
    assert!(demo_proj_loc.contains("/projects/"));
    let demo_proj_id = demo_proj_loc.split('/').nth(2).unwrap().split('?').next().unwrap();

    // 3. Verify Files Tab is Default View on Project Page
    let proj_files_res = client.get(format!("{}/projects/{}", base_url, demo_proj_id)).send().await.unwrap();
    assert_eq!(proj_files_res.status(), StatusCode::OK);
    let files_html = proj_files_res.text().await.unwrap();
    assert!(files_html.contains("slides.typ"));
    assert!(files_html.contains("paper.typ"));
    assert!(files_html.contains("quantum_measurements.table"));
    assert!(files_html.contains("lab_notebook.anote"));

    // 4. Verify Sharing & Permissions Tab with the 3 Strict Collaboration Roles
    let sharing_res = client.get(format!("{}/projects/{}?tab=members", base_url, demo_proj_id)).send().await.unwrap();
    assert_eq!(sharing_res.status(), StatusCode::OK);
    let sharing_html = sharing_res.text().await.unwrap();
    assert!(sharing_html.contains("read_only"));
    assert!(sharing_html.contains("read_and_review"));
    assert!(sharing_html.contains("read_write_and_review"));
    assert!(sharing_html.contains("Copy Share Link"));

    // 5. Test Dedicated Document & Slide Editor Studio
    let editor_res = client.get(format!("{}/projects/{}/editor?file=slides.typ", base_url, demo_proj_id)).send().await.unwrap();
    assert_eq!(editor_res.status(), StatusCode::OK);
    let editor_html = editor_res.text().await.unwrap();
    assert!(editor_html.contains("slides.typ"));
    assert!(editor_html.contains("slide-stage") || editor_html.contains("slide-theme"));

    // Save document edit
    let save_doc_res = client
        .post(format!("{}/projects/{}/editor/save", base_url, demo_proj_id))
        .form(&[
            ("file", "slides.typ"),
            ("content", "// Updated slides with quantum coherence\n#import \"theme.typ\": *\n"),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(save_doc_res.status(), StatusCode::SEE_OTHER);

    // 6. Test Spreadsheet Table: Cell Edit, Row Add, Row Delete, CSV Export & Import
    let cell_edit_res = client
        .post(format!("{}/projects/{}/table/cell-edit", base_url, demo_proj_id))
        .form(&[
            ("file", "quantum_measurements.table"),
            ("table", "qubit_characterization"),
            ("row_id_col", "id"),
            ("row_id_val", "1"),
            ("col", "qubit_label"),
            ("val", "Q0_Transmon_Calibrated_v2"),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(cell_edit_res.status(), StatusCode::OK);

    // Export CSV
    let export_csv_res = client
        .get(format!("{}/projects/{}/table/export?file=quantum_measurements.table&table=qubit_characterization", base_url, demo_proj_id))
        .send()
        .await
        .unwrap();
    assert_eq!(export_csv_res.status(), StatusCode::OK);
    let exported_csv = export_csv_res.text().await.unwrap();
    assert!(exported_csv.contains("Q0_Transmon_Calibrated_v2"));

    // Import CSV
    let import_csv_res = client
        .post(format!("{}/projects/{}/table/import", base_url, demo_proj_id))
        .form(&[
            ("file", "quantum_measurements.table"),
            ("table", "qubit_characterization"),
            ("csv_data", "id,qubit_label,frequency_ghz,t1_us,t2_echo_us,readout_fidelity,status\n10,Q9_Coupled,5.42,99.1,80.2,0.994,Online\n"),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(import_csv_res.status(), StatusCode::SEE_OTHER);

    // 7. Test Dedicated Unified Note Studio (.anote)
    let note_studio_res = client
        .get(format!("{}/projects/{}/note?file=lab_notebook.anote&view=editor", base_url, demo_proj_id))
        .send()
        .await
        .unwrap();
    assert_eq!(note_studio_res.status(), StatusCode::OK);
    let note_html = note_studio_res.text().await.unwrap();
    assert!(note_html.contains("Document Outline"));
    assert!(note_html.contains("Save Note"));

    // Whiteboard Canvas View in Unified Note Studio
    let wb_res = client
        .get(format!("{}/projects/{}/note?file=lab_notebook.anote&view=whiteboard", base_url, demo_proj_id))
        .send()
        .await
        .unwrap();
    assert_eq!(wb_res.status(), StatusCode::OK);
    let wb_html = wb_res.text().await.unwrap();
    // The whiteboard is a real Leptos island (apich-islands::WhiteboardIsland) hydrated with
    // real Rust/WASM in the browser, not a hand-written JS canvas script -- verify the server
    // actually emitted the island marker with a real <canvas> inside, not a hardcoded element id.
    assert!(wb_html.contains("leptos-island"));
    assert!(wb_html.contains("data-component=\"WhiteboardIsland_"));
    assert!(wb_html.contains("<canvas"));
    assert!(wb_html.contains("Export PNG"));

    // Save Note Form
    let save_note_res = client
        .post(format!("{}/projects/{}/note/save", base_url, demo_proj_id))
        .form(&[
            ("file", "lab_notebook.anote"),
            ("view", "editor"),
            ("meta_title", "Cryogenic Qubit Characterization Notebook"),
            ("meta_author", "Alice & Bob"),
            ("meta_tags", "quantum, dilution-fridge, transmon"),
            ("body", "# Lab Notebook\n\n## Next Steps\n- [ ] Calibrate pulse envelope\n"),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(save_note_res.status(), StatusCode::SEE_OTHER);

    // 7.6 Logout Flow: POST /logout redirects to /login and clears session cookie
    let logout_res = client.post(format!("{}/logout", base_url)).send().await.unwrap();
    assert_eq!(logout_res.status(), StatusCode::SEE_OTHER);
    assert_eq!(logout_res.headers().get("location").unwrap(), "/login");

    println!("All fullstack isomorphic integration tests completed successfully!");

    // Clean up test postgres container
    let _ = container.stop(5).await;
}
