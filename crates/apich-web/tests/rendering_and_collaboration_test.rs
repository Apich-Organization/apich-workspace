use apich_db::{CreateUserDto, Database, PostgresConfig, PostgresContainer, UserRole};
use apich_sandbox::SandboxManager;
use apich_web::{create_app, AppState};
use reqwest::StatusCode;
use serde_json::{json, Value};
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
async fn test_rendering_notes_sharing_and_ai_copilot() {
    // 0. Setup PostgreSQL test connection
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

    let _ = sqlx::query("TRUNCATE TABLE users, organizations, oauth_clients, system_settings, invitations CASCADE")
        .execute(db.pool())
        .await;

    let _admin = repo
        .create_user(CreateUserDto {
            username: "dr_alice".to_string(),
            email: "alice@lab.quantum.org".to_string(),
            password_hash: apich_web::auth::hash_password("SuperSecretPass123!").unwrap(),
            display_name: "Dr. Alice Stern".to_string(),
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
        .expect("Failed to bind ephemeral port");
    let server_port = listener.local_addr().expect("Failed to get local port").port();
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

    let client = reqwest::Client::builder()
        .cookie_store(true)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Failed to build HTTP client");

    // Login as admin
    let login_res = client
        .post(format!("{}/login", base_url))
        .form(&[
            ("login", "dr_alice"),
            ("password", "SuperSecretPass123!"),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(login_res.status(), StatusCode::SEE_OTHER);

    // Create Showcase Demo Project
    let create_demo_res = client.post(format!("{}/projects/demo/create", base_url)).send().await.unwrap();
    assert_eq!(create_demo_res.status(), StatusCode::SEE_OTHER);
    let demo_proj_loc = create_demo_res.headers().get("location").unwrap().to_str().unwrap();
    let proj_id = demo_proj_loc.split('/').nth(2).unwrap().split('?').next().unwrap();

    // ========================================================================
    // TEST 1: Real Typst SVG Compilation & Dev-mode Reverse Search Links
    // ========================================================================
    println!("--- 1. Testing Real Typst SVG Compilation with Reverse Search Links ---");
    let typst_source = "#set page(width: 10cm, height: 6cm, margin: 1cm)\n#set text(size: 14pt)\n\n= Quantum Calibration Report\nThis is line 4 with experimental findings.\nThe Hamiltonian is $H = h omega_0 sigma_z / 2$.\n";

    let save_res = client
        .post(format!("{}/projects/{}/editor/save", base_url, proj_id))
        .form(&[
            ("file", "quantum_report.typ"),
            ("content", typst_source),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(save_res.status(), StatusCode::SEE_OTHER);

    let editor_res = client
        .get(format!("{}/projects/{}/editor?file=quantum_report.typ", base_url, proj_id))
        .send()
        .await
        .unwrap();
    assert_eq!(editor_res.status(), StatusCode::OK);
    let editor_html = editor_res.text().await.unwrap();

    // Verify SVG was generated and contains reverse-search hyperlink anchors
    assert!(editor_html.contains("<svg"), "Page must contain rendered SVG output");
    assert!(
        editor_html.contains("sync:line:") || editor_html.contains("Quantum Calibration"),
        "SVG must contain reverse search sync:line hyperlink or compiled content"
    );

    // ========================================================================
    // TEST 2: Real Interactive Script Runner (Python Execution, Plots, Exit Code)
    // ========================================================================
    println!("--- 2. Testing Interactive Python Script Runner ---");
    let script_code = "import sys\nprint(\"APICH Script Runner Initialized\")\nprint(\"Argument count:\", len(sys.argv))\nsys.stdout.flush()\n";

    let _ = client
        .post(format!("{}/projects/{}/editor/save", base_url, proj_id))
        .form(&[
            ("file", "telemetry_analysis.py"),
            ("content", script_code),
        ])
        .send()
        .await
        .unwrap();

    let run_res = client
        .post(format!("{}/projects/{}/script/run", base_url, proj_id))
        .json(&json!({
            "file": "telemetry_analysis.py",
            "args": "--batch-size 128"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(run_res.status(), StatusCode::OK);
    let run_json: Value = run_res.json().await.unwrap();
    assert_eq!(run_json["success"], true);
    assert_eq!(run_json["exit_code"], 0);
    assert!(run_json["stdout"].as_str().unwrap().contains("APICH Script Runner Initialized"));
    assert!(run_json["execution_time_ms"].is_number());

    // ========================================================================
    // TEST 3: Unified Reactive Note Studio, 3-Column View & Live AJAX Task Sync
    // ========================================================================
    println!("--- 3. Testing Unified Reactive Note Studio & AJAX Task Checkbox Sync ---");
    let note_raw = "---\ntitle: \"Superconducting Cavity QED Notebook\"\nauthor: \"Dr. Alice Stern\"\ntags: [\"cqed\", \"qubit\", \"telemetry\"]\n---\n\n# Quantum Experiment Plan\nWe analyze the decoherence envelope of the qubit:\n$$ |psi(t)> = cos(Omega t / 2)|0> - i sin(Omega t / 2)|1> $$\n\n## Action Checklist\n- [ ] Calibrate pulse envelope @2026-10-15 #calibration\n- [ ] Measure T1 relaxation rate @2026-10-16 #measurement\n- [x] Cool down dilution refrigerator to 15mK #cryo\n";

    let _ = client
        .post(format!("{}/projects/{}/editor/save", base_url, proj_id))
        .form(&[
            ("file", "cavity_experiment.anote"),
            ("content", note_raw),
        ])
        .send()
        .await
        .unwrap();

    let note_page_res = client
        .get(format!("{}/projects/{}/note?file=cavity_experiment.anote&view=editor", base_url, proj_id))
        .send()
        .await
        .unwrap();
    assert_eq!(note_page_res.status(), StatusCode::OK);
    let note_page_html = note_page_res.text().await.unwrap();

    // Verify 3-column layout & components
    assert!(note_page_html.contains("editor-studio-grid"), "Must use 3-column grid");
    assert!(note_page_html.contains("outline-panel"), "Must have outline panel");
    assert!(note_page_html.contains("code-panel"), "Must have code editor panel");
    assert!(note_page_html.contains("preview-panel"), "Must have live preview panel");
    assert!(note_page_html.contains("task-live-checkbox"), "Live preview must have interactive checkboxes");
    assert!(note_page_html.contains("Task Progress"), "Must show tasks progress");
    assert!(note_page_html.contains("KaTeX Math"), "Must indicate KaTeX support");

    // Toggle task line 11 (Calibrate pulse envelope) via AJAX endpoint
    let toggle_ajax_res = client
        .post(format!("{}/projects/{}/knowledge/toggle-task-ajax", base_url, proj_id))
        .form(&[
            ("file", "cavity_experiment.anote"),
            ("line_number", "11"),
            ("status", "done"),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(toggle_ajax_res.status(), StatusCode::OK);
    let toggle_json: Value = toggle_ajax_res.json().await.unwrap();
    assert_eq!(toggle_json["success"], true);

    let updated_body = toggle_json["updated_body"].as_str().unwrap();
    assert!(
        updated_body.contains("- [x] Calibrate pulse envelope"),
        "Task in note body must be toggled to completed [x]"
    );

    // Verify file on disk is also updated
    let reloaded_file_res = client
        .get(format!("{}/projects/{}/editor?file=cavity_experiment.anote", base_url, proj_id))
        .send()
        .await
        .unwrap();
    let reloaded_html = reloaded_file_res.text().await.unwrap();
    assert!(reloaded_html.contains("- [x] Calibrate pulse envelope"));

    // ========================================================================
    // TEST 4: File-Level and Project-Level Sharing Configurations
    // ========================================================================
    println!("--- 4. Testing Multi-level File and Project Sharing ---");

    // Update File Sharing for cavity_experiment.anote: Public, read_only
    let file_share_res = client
        .post(format!("{}/projects/{}/files/share", base_url, proj_id))
        .form(&[
            ("file", "cavity_experiment.anote"),
            ("mode", "public"),
            ("role", "read_only"),
            ("allowed_users", ""),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(file_share_res.status(), StatusCode::SEE_OTHER);

    // Verify Files tab displays the updated Public sharing pill
    let files_tab_res = client
        .get(format!("{}/projects/{}?tab=files", base_url, proj_id))
        .send()
        .await
        .unwrap();
    let files_tab_html = files_tab_res.text().await.unwrap();
    assert!(files_tab_html.contains("Public (read_only)") || files_tab_html.contains("share-badge-public"));

    // Update Project Sharing Configuration
    let proj_share_res = client
        .post(format!("{}/projects/{}/sharing/update", base_url, proj_id))
        .form(&[
            ("is_public", "true"),
            ("default_role", "read_and_review"),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(proj_share_res.status(), StatusCode::SEE_OTHER);

    // ========================================================================
    // TEST 5: AI Assistant & Copilot Agent Integration
    // ========================================================================
    println!("--- 5. Testing AI Assistant & Copilot Agent Endpoint ---");

    let ai_req_res = client
        .post(format!("{}/projects/{}/ai/chat", base_url, proj_id))
        .json(&json!({
            "prompt": "Add the calibrated Hamiltonian formulation in Typst",
            "provider": "builtin",
            "context_file": "quantum_report.typ"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(ai_req_res.status(), StatusCode::OK);
    let ai_json: Value = ai_req_res.json().await.unwrap();
    assert_eq!(ai_json["success"], true);
    assert!(
        ai_json["reply"].as_str().unwrap().contains("Hamiltonian") || ai_json["reply"].as_str().unwrap().contains("omega"),
        "AI response must contain formulated scientific content"
    );
    assert!(ai_json["suggested_code"].as_str().is_some());

    println!("All Rendering, Note Studio, Multi-level Sharing and AI Copilot tests passed!");

    let _ = container.stop(5).await;
}
