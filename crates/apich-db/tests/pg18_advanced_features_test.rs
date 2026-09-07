mod common;
use apich_db::{
    CreateDocumentDto, CreateUserDto, CreateWorkspaceDto, Database, DocType, PostgresConfig,
    PostgresContainer, WorkspaceVisibility,
};
use common::test_temp_dir;
use std::time::Duration;

#[tokio::test]
async fn test_pg18_advanced_features() {
    let temp_data = test_temp_dir();
    let temp_backup = test_temp_dir();

    let config = PostgresConfig::builder(temp_data.path(), temp_backup.path())
        .container_name("apich-pg18-features-test")
        .host_port(5437)
        .database("test_pg18_db")
        .admin_user("postgres")
        .admin_password("admin_pg18_pwd")
        .selinux_relabel(true)
        .build();

    let container = PostgresContainer::new(config.clone());
    container
        .ensure_running()
        .await
        .expect("Failed to start PG18 container");
    container
        .wait_ready(Duration::from_secs(30))
        .await
        .expect("Postgres 18 not ready");

    let db = Database::connect_admin(&config, "localhost")
        .await
        .expect("Failed to connect admin");

    db.run_migrations().await.expect("Failed to run migrations");

    let pool = db.pool();
    let repo = db.repository();

    // 1. Verify PostgreSQL 18 version
    let pg_version: String = sqlx::query_scalar("SELECT version();")
        .fetch_one(pool)
        .await
        .expect("Failed to query pg version");
    println!("PostgreSQL Version: {}", pg_version);
    assert!(
        pg_version.contains("PostgreSQL 18"),
        "Expected PostgreSQL 18, got: {}",
        pg_version
    );

    // 2. Setup user and workspace
    let user = repo
        .create_user(CreateUserDto {
            username: "dr_researcher".to_string(),
            email: "researcher@lab.org".to_string(),
            password_hash: "argon2id_secret".to_string(),
            display_name: "Dr. Researcher".to_string(),
            role: None,
            is_platform_admin: None,
            storage_quota_bytes: None,
        })
        .await
        .expect("Failed to create user");

    let ws = repo
        .create_workspace(CreateWorkspaceDto {
            id: None,
            owner_id: user.id,
            slug: "ai-systems".to_string(),
            name: "AI & Distributed Systems Lab".to_string(),
            description: Some("Deep learning and compiler optimization lab".to_string()),
            visibility: Some(WorkspaceVisibility::Internal),
            settings: Some(serde_json::json!({
                "theme": "academic-dark",
                "latex_engine": "lualatex",
                "typst_version": "0.13"
            })),
        })
        .await
        .expect("Failed to create workspace");

    // 3. Test PostgreSQL 18 Native UUIDv7 default generation and extraction
    let doc1 = repo
        .create_document(CreateDocumentDto {
            id: None, // Will use UUIDv7 generated in Rust or PG18 default
            workspace_id: ws.id,
            rel_path: "distributed_consensus.typ".to_string(),
            title: "Raft and Paxos Distributed Consensus Protocol".to_string(),
            content: Some(
                "Distributed algorithms enable state machine replication across unreliable networks. Fault tolerance requires quorum consensus.".to_string(),
            ),
            doc_type: Some(DocType::Typst),
            metadata: Some(serde_json::json!({
                "tags": ["distributed-systems", "consensus", "typst"],
                "keywords": ["raft", "paxos", "quorum"],
                "citation_count": 42
            })),
        })
        .await
        .expect("Failed to create doc1");

    let doc2 = repo
        .create_document(CreateDocumentDto {
            id: None,
            workspace_id: ws.id,
            rel_path: "quantum_crypto.md".to_string(),
            title: "Post-Quantum Cryptography and Kyber Lattice KEM".to_string(),
            content: Some(
                "Lattice-based cryptography resists Shor's algorithm on quantum computers. Kyber is standard for key encapsulation.".to_string(),
            ),
            doc_type: Some(DocType::Markdown),
            metadata: Some(serde_json::json!({
                "tags": ["cryptography", "quantum", "security"],
                "keywords": ["lattice", "kyber", "kem"],
                "citation_count": 18
            })),
        })
        .await
        .expect("Failed to create doc2");

    // Test PG18 uuid_extract_timestamp
    let extracted_ts = repo
        .extract_uuidv7_timestamp(doc1.id)
        .await
        .expect("Failed to extract UUIDv7 timestamp via PG18");
    println!("Extracted creation timestamp from UUIDv7: {}", extracted_ts);
    assert_eq!(
        extracted_ts.timestamp(),
        doc1.created_at.timestamp(),
        "Extracted timestamp should match created_at"
    );

    // 4. Test PostgreSQL 18 Generated tsvector & GIN Full-Text Search
    let search_consensus = repo
        .search_documents_fulltext(ws.id, "consensus quorum", 10)
        .await
        .expect("Failed full-text search for consensus");
    assert_eq!(search_consensus.len(), 1);
    assert_eq!(search_consensus[0].id, doc1.id);
    assert!(search_consensus[0].rank > 0.0);
    println!(
        "FTS Result 1: '{}' with rank: {}",
        search_consensus[0].title, search_consensus[0].rank
    );

    let search_quantum = repo
        .search_documents_fulltext(ws.id, "quantum lattice", 10)
        .await
        .expect("Failed full-text search for quantum");
    assert_eq!(search_quantum.len(), 1);
    assert_eq!(search_quantum[0].id, doc2.id);
    assert!(search_quantum[0].rank > 0.0);
    println!(
        "FTS Result 2: '{}' with rank: {}",
        search_quantum[0].title, search_quantum[0].rank
    );

    // 5. Test JSONB GIN jsonb_path_ops index with containment query (@>)
    let filter = serde_json::json!({ "tags": ["cryptography"] });
    let docs_by_tag = repo
        .find_documents_by_metadata(ws.id, &filter)
        .await
        .expect("Failed to find docs by metadata containment");
    assert_eq!(docs_by_tag.len(), 1);
    assert_eq!(docs_by_tag[0].id, doc2.id);

    // 6. Test PostgreSQL 18 JSON_TABLE function
    let tags = repo
        .extract_document_tags(doc1.id)
        .await
        .expect("Failed to query tags via JSON_TABLE");
    println!("Extracted tags via JSON_TABLE: {:?}", tags);
    assert_eq!(tags, vec!["distributed-systems", "consensus", "typst"]);

    // 7. Test Atomic Upsert (MERGE / ON CONFLICT)
    let upsert_dto = CreateDocumentDto {
        id: None,
        workspace_id: ws.id,
        rel_path: "distributed_consensus.typ".to_string(),
        title: "Raft and Paxos Distributed Consensus Protocol (Revised Edition)".to_string(),
        content: Some("Revised with Byzantine Fault Tolerance (BFT) extensions.".to_string()),
        doc_type: Some(DocType::Typst),
        metadata: Some(serde_json::json!({
            "tags": ["distributed-systems", "consensus", "bft"],
            "citation_count": 55
        })),
    };
    let updated_doc = repo
        .upsert_document(upsert_dto)
        .await
        .expect("Failed to upsert document");
    assert_eq!(updated_doc.id, doc1.id);
    assert_eq!(
        updated_doc.title,
        "Raft and Paxos Distributed Consensus Protocol (Revised Edition)"
    );
    assert_eq!(updated_doc.version, 2);
    assert_eq!(
        updated_doc.content,
        "Revised with Byzantine Fault Tolerance (BFT) extensions."
    );

    // Search revised document content via FTS
    let search_bft = repo
        .search_documents_fulltext(ws.id, "Byzantine", 10)
        .await
        .expect("Failed full-text search for updated content");
    assert_eq!(search_bft.len(), 1);
    assert_eq!(search_bft[0].id, doc1.id);

    // Cleanup
    drop(db);
    container
        .destroy()
        .await
        .expect("Failed to destroy container");
}
