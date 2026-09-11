mod common;
use apich_db::CreateAuditLogDto;
use apich_db::CreateDocumentDto;
use apich_db::CreateKnowledgeEdgeDto;
use apich_db::CreateKnowledgeNodeDto;
use apich_db::CreateUserDto;
use apich_db::CreateWorkspaceDto;
use apich_db::Database;
use apich_db::DocType;
use apich_db::MemberRole;
use apich_db::PostgresConfig;
use apich_db::PostgresContainer;
use apich_db::UserRole;
use apich_db::WorkspaceVisibility;
use common::test_temp_dir;
use std::time::Duration;

#[tokio::test]
async fn test_models_and_schema_migrations() {
    let temp_data = test_temp_dir();
    let temp_backup = test_temp_dir();

    let config = PostgresConfig::builder(temp_data.path(), temp_backup.path())
        .container_name("apich-pg-models-test")
        .host_port(5436)
        .database("test_models_db")
        .admin_user("postgres")
        .admin_password("admin_models_pwd")
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

    // 1. Connect admin and run migrations
    let db = Database::connect_admin(&config, "localhost")
        .await
        .expect("Failed to connect admin");

    db.run_migrations().await.expect("Failed to run migrations");

    // Idempotent migration check
    db.run_migrations()
        .await
        .expect("Re-running migrations should be idempotent");

    let repo = db.repository();

    // 2. User entity tests
    let user_dto = CreateUserDto {
        username: "alice".to_string(),
        email: "alice@apich.org".to_string(),
        password_hash: "argon2id_hash_placeholder".to_string(),
        display_name: "Alice Wang".to_string(),
        role: Some(UserRole::Admin),
        is_platform_admin: None,
        storage_quota_bytes: Some(20 * 1024 * 1024 * 1024), // 20 GB
    };
    let user_alice = repo
        .create_user(user_dto)
        .await
        .expect("Failed to create user");
    assert_eq!(user_alice.username, "alice");
    assert_eq!(user_alice.role, UserRole::Admin);
    assert_eq!(user_alice.storage_quota_bytes, 20 * 1024 * 1024 * 1024);

    let fetched_user = repo
        .get_user_by_username("alice")
        .await
        .expect("Failed to fetch user")
        .expect("User not found");
    assert_eq!(fetched_user.id, user_alice.id);

    let bob_dto = CreateUserDto {
        username: "bob".to_string(),
        email: "bob@apich.org".to_string(),
        password_hash: "hash_bob".to_string(),
        display_name: "Bob Li".to_string(),
        role: Some(UserRole::Member),
        is_platform_admin: None,
        storage_quota_bytes: None,
    };
    let user_bob = repo
        .create_user(bob_dto)
        .await
        .expect("Failed to create bob");

    // 3. Workspace entity tests
    let ws_dto = CreateWorkspaceDto {
        id: None,
        owner_id: user_alice.id,
        slug: "quantum-computing".to_string(),
        name: "Quantum Computing Paper".to_string(),
        description: Some("Academic research workspace for quantum systems".to_string()),
        visibility: Some(WorkspaceVisibility::Private),
        settings: None,
    };
    let ws = repo
        .create_workspace(ws_dto)
        .await
        .expect("Failed to create workspace");
    assert_eq!(ws.slug, "quantum-computing");
    assert_eq!(ws.owner_id, user_alice.id);
    assert_eq!(
        ws.container_name.as_deref(),
        Some("apich-ws-quantum-computing")
    );

    // Add Bob as Contributor
    let member = repo
        .add_workspace_member(ws.id, user_bob.id, MemberRole::Contributor)
        .await
        .expect("Failed to add member");
    assert_eq!(member.role, MemberRole::Contributor);

    // 4. Document entity tests
    let doc_dto = CreateDocumentDto {
        id: None,
        workspace_id: ws.id,
        rel_path: "papers/main.typ".to_string(),
        title: "Main Paper (Typst)".to_string(),
        content: Some("= Quantum Mechanics\n\nIntro to quantum theory.".to_string()),
        doc_type: Some(DocType::Typst),
        metadata: Some(serde_json::json!({
            "engine": "typst",
            "version": "0.13.0"
        })),
    };
    let doc = repo
        .create_document(doc_dto)
        .await
        .expect("Failed to create document");
    assert_eq!(doc.rel_path, "papers/main.typ");
    assert_eq!(doc.doc_type, DocType::Typst);
    assert_eq!(
        doc.content,
        "= Quantum Mechanics\n\nIntro to quantum theory."
    );

    let fetched_doc = repo
        .get_document_by_path(ws.id, "papers/main.typ")
        .await
        .expect("Failed to fetch doc")
        .expect("Doc not found");
    assert_eq!(fetched_doc.id, doc.id);

    // 5. Knowledge Graph tests (Section 4: Knowledge Graph)
    let node1 = repo
        .create_knowledge_node(CreateKnowledgeNodeDto {
            id: None,
            workspace_id: ws.id,
            node_type: "wiki".to_string(),
            title: "Quantum Superposition".to_string(),
            content_hash: Some("sha256_hash_123".to_string()),
            metadata: Some(serde_json::json!({ "tags": ["physics", "quantum"] })),
        })
        .await
        .expect("Failed to create knowledge node 1");

    let node2 = repo
        .create_knowledge_node(CreateKnowledgeNodeDto {
            id: None,
            workspace_id: ws.id,
            node_type: "task".to_string(),
            title: "Verify Bloch Sphere Simulation".to_string(),
            content_hash: None,
            metadata: Some(serde_json::json!({ "status": "in_progress", "priority": "high" })),
        })
        .await
        .expect("Failed to create knowledge node 2");

    let edge = repo
        .create_knowledge_edge(CreateKnowledgeEdgeDto {
            id: None,
            workspace_id: ws.id,
            source_id: node2.id,
            target_id: node1.id,
            relation_type: "depends_on".to_string(),
            weight: Some(2.5),
        })
        .await
        .expect("Failed to create knowledge edge");
    assert_eq!(edge.relation_type, "depends_on");
    assert_eq!(edge.weight, 2.5);

    // 6. Audit Log tests (Section 6: Audit Trail)
    let log = repo
        .record_audit_log(CreateAuditLogDto {
            user_id: Some(user_alice.id),
            workspace_id: Some(ws.id),
            action: "workspace.document.publish".to_string(),
            details: serde_json::json!({
                "document_id": doc.id,
                "format": "pdf"
            }),
            ip_address: Some("127.0.0.1".to_string()),
        })
        .await
        .expect("Failed to record audit log");
    assert_eq!(log.action, "workspace.document.publish");

    // Cleanup
    drop(db);
    container
        .destroy()
        .await
        .expect("Failed to destroy container");
}
