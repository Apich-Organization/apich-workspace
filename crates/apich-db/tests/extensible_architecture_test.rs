mod common;
use apich_db::CreateDocumentDto;
use apich_db::CreateUserDto;
use apich_db::CreateWorkspaceDto;
use apich_db::Database;
use apich_db::DocType;
use apich_db::ExtensibleMetadata;
use apich_db::Migration;
use apich_db::MigrationManager;
use apich_db::PostgresConfig;
use apich_db::PostgresContainer;
use apich_db::WorkspaceVisibility;
use common::test_temp_dir;
use serde::Deserialize;
use serde::Serialize;
use std::time::Duration;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct GitExtensionMeta {
    pub repo_url: String,
    pub default_branch: String,
    pub commit_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct TypstCompilerMeta {
    pub cache_key: String,
    pub font_family: String,
    pub page_count: u32,
}

#[tokio::test]
async fn test_extensible_architecture_and_rls() {
    let temp_data = test_temp_dir();
    let temp_backup = test_temp_dir();

    let config = PostgresConfig::builder(temp_data.path(), temp_backup.path())
        .container_name("apich-pg18-extensible-test")
        .host_port(5438)
        .database("test_extensible_db")
        .admin_user("postgres")
        .admin_password("admin_ext_pwd")
        .app_user("apich_app")
        .app_password("app_ext_pwd")
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
        .expect("Postgres 18 not ready");

    // 1. Test Extensible Migration Registration
    // Simulate another crate (e.g. `apich-git`) registering its own schema migration!
    let mut migration_manager = MigrationManager::new();
    migration_manager.register(Migration {
        version: 10,
        name: "010_git_integration_schema",
        sql: r#"
        CREATE TABLE IF NOT EXISTS git_repositories (
            id UUID PRIMARY KEY DEFAULT uuidv7(),
            workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
            remote_url TEXT NOT NULL,
            current_branch VARCHAR(128) NOT NULL DEFAULT 'main',
            is_bare BOOLEAN NOT NULL DEFAULT false,
            created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE INDEX IF NOT EXISTS idx_git_repo_workspace ON git_repositories(workspace_id);
        "#,
    });

    let mut db = Database::connect_admin(&config, "localhost")
        .await
        .expect("Failed to connect admin");

    // Run core migrations + external module migration
    let applied = db
        .migrate_with(&migration_manager)
        .await
        .expect("Failed to run extensible migrations");
    assert!(applied.len() >= 3);
    println!("Applied migrations count: {}", applied.len());

    // Setup least-privilege application role permissions
    db.setup_permissions(&config)
        .await
        .expect("Failed to setup permissions");

    // Connect app pool
    db.init_app_pool(&config, "localhost")
        .await
        .expect("Failed to connect app pool");

    let repo = db.repository();

    // 2. Test Extensible Metadata Pattern on Entities
    let alice = repo
        .create_user(CreateUserDto {
            username: "alice_crypto".to_string(),
            email: "alice@zkp.org".to_string(),
            password_hash: "hash123".to_string(),
            display_name: "Alice ZKP".to_string(),
            role: None,
            is_platform_admin: None,
            storage_quota_bytes: None,
        })
        .await
        .expect("Failed to create alice");

    let bob = repo
        .create_user(CreateUserDto {
            username: "bob_open".to_string(),
            email: "bob@open.org".to_string(),
            password_hash: "hash456".to_string(),
            display_name: "Bob Open".to_string(),
            role: None,
            is_platform_admin: None,
            storage_quota_bytes: None,
        })
        .await
        .expect("Failed to create bob");

    // Create Alice's private workspace
    let mut ws_alice = repo
        .create_workspace(CreateWorkspaceDto {
            id: None,
            owner_id: alice.id,
            slug: "alice-zkp-paper".to_string(),
            name: "Alice Zero-Knowledge Proofs".to_string(),
            description: Some("Confidential research".to_string()),
            visibility: Some(WorkspaceVisibility::Private),
            settings: None,
        })
        .await
        .expect("Failed to create alice ws");

    // Attach strongly-typed Git workspace metadata using ExtensibleMetadata trait
    let git_config = GitExtensionMeta {
        repo_url: "git@github.com:alice/zkp-research.git".to_string(),
        default_branch: "main".to_string(),
        commit_hash: "a1b2c3d4e5f6".to_string(),
    };
    ws_alice
        .set_ext("git", &git_config)
        .expect("Failed to set git ext");
    let retrieved_git: Option<GitExtensionMeta> = ws_alice.get_ext("git");
    assert_eq!(retrieved_git, Some(git_config));

    // Create Alice's confidential document
    let mut doc_alice = repo
        .create_document(CreateDocumentDto {
            id: None,
            workspace_id: ws_alice.id,
            rel_path: "zkp_circuits.typ".to_string(),
            title: "Zero Knowledge SNARK Circuits".to_string(),
            content: Some("Private circuit constraints for groth16 protocol.".to_string()),
            doc_type: Some(DocType::Typst),
            metadata: None,
        })
        .await
        .expect("Failed to create alice doc");

    // Attach strongly-typed Typst compiler metadata
    let typst_meta = TypstCompilerMeta {
        cache_key: "cache_sha_987".to_string(),
        font_family: "Libertinus Serif".to_string(),
        page_count: 14,
    };
    doc_alice
        .set_ext("typst_cache", &typst_meta)
        .expect("Failed to set typst cache ext");
    let retrieved_typst: Option<TypstCompilerMeta> = doc_alice.get_ext("typst_cache");
    assert_eq!(retrieved_typst, Some(typst_meta));

    // Create Bob's public workspace & document
    let ws_bob = repo
        .create_workspace(CreateWorkspaceDto {
            id: None,
            owner_id: bob.id,
            slug: "bob-open-data".to_string(),
            name: "Bob Open Science Dataset".to_string(),
            description: Some("Open source datasets".to_string()),
            visibility: Some(WorkspaceVisibility::Public),
            settings: None,
        })
        .await
        .expect("Failed to create bob ws");

    let doc_bob = repo
        .create_document(CreateDocumentDto {
            id: None,
            workspace_id: ws_bob.id,
            rel_path: "datasets/index.md".to_string(),
            title: "Public Open Science Index".to_string(),
            content: Some("Open access climate and satellite imaging datasets.".to_string()),
            doc_type: Some(DocType::Markdown),
            metadata: None,
        })
        .await
        .expect("Failed to create bob doc");

    // Verify external table dynamically created by Migration 10
    let repo_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO git_repositories (id, workspace_id, remote_url, current_branch) VALUES ($1, $2, $3, $4)"
    )
    .bind(repo_id)
    .bind(ws_alice.id)
    .bind("git@github.com:alice/zkp-research.git")
    .bind("develop")
    .execute(db.pool())
    .await
    .expect("Failed to insert into dynamically registered git_repositories table");

    let branch: String =
        sqlx::query_scalar("SELECT current_branch FROM git_repositories WHERE id = $1")
            .bind(repo_id)
            .fetch_one(db.pool())
            .await
            .expect("Failed to query git_repositories");
    assert_eq!(branch, "develop");

    // 3. Test Row-Level Security (RLS) Isolation via DbSession
    // 3.1 Alice's Session:
    let mut alice_session = db
        .begin_session(Some(alice.id))
        .await
        .expect("Failed to begin alice session");
    let alice_visible_ws = alice_session
        .list_visible_workspaces()
        .await
        .expect("Failed to list visible ws for alice");
    // Alice sees her private workspace AND Bob's public workspace
    assert_eq!(alice_visible_ws.len(), 2);

    let alice_docs = alice_session
        .list_visible_documents(ws_alice.id)
        .await
        .expect("Failed to list alice docs");
    assert_eq!(alice_docs.len(), 1);
    assert_eq!(alice_docs[0].id, doc_alice.id);
    alice_session.commit().await.unwrap();

    // 3.2 Bob's Session:
    let mut bob_session = db
        .begin_session(Some(bob.id))
        .await
        .expect("Failed to begin bob session");
    let bob_visible_ws = bob_session
        .list_visible_workspaces()
        .await
        .expect("Failed to list visible ws for bob");
    // Bob should ONLY see his public workspace, NOT Alice's private workspace!
    assert_eq!(bob_visible_ws.len(), 1);
    assert_eq!(bob_visible_ws[0].id, ws_bob.id);

    // Bob attempts to read Alice's confidential document
    let bob_attempts_alice_doc = bob_session
        .get_visible_document_by_path(ws_alice.id, "zkp_circuits.typ")
        .await
        .expect("Query failed");
    assert!(
        bob_attempts_alice_doc.is_none(),
        "RLS MUST prevent Bob from accessing Alice's private document!"
    );

    // Bob can read his own public document
    let bob_own_doc = bob_session
        .get_visible_document_by_path(ws_bob.id, "datasets/index.md")
        .await
        .expect("Query failed");
    assert!(bob_own_doc.is_some());
    assert_eq!(bob_own_doc.unwrap().id, doc_bob.id);
    bob_session.commit().await.unwrap();

    // 3.3 System Session (RLS Bypassed):
    let mut sys_session = db
        .begin_system_session()
        .await
        .expect("Failed to begin system session");
    let sys_visible_ws = sys_session
        .list_visible_workspaces()
        .await
        .expect("Failed to list all workspaces");
    assert_eq!(sys_visible_ws.len(), 2);
    sys_session.commit().await.unwrap();

    // Cleanup
    drop(db);
    container
        .destroy()
        .await
        .expect("Failed to destroy container");
}
