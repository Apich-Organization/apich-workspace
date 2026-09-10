mod common;

use apich_db::{
    CreateInvitationDto, CreateOAuthClientDto, CreateOrganizationDto, CreateProjectDto,
    CreateTeamDto, CreateUserDto, Database, IdentityPermissionResolver, PostgresConfig,
    PostgresContainer, UserRole,
};
use common::test_temp_dir;
use std::time::Duration;

#[tokio::test]
async fn test_identity_tree_hierarchy_permissions_and_lifecycle() {
    let temp_data = test_temp_dir();
    let temp_backup = test_temp_dir();

    let config = PostgresConfig::builder(temp_data.path(), temp_backup.path())
        .container_name("apich-pg-identity-test")
        .host_port(5437)
        .database("test_identity_db")
        .admin_user("postgres")
        .admin_password("admin_identity_pwd")
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

    let db = Database::connect_admin(&config, "localhost")
        .await
        .expect("Failed to connect admin");

    db.run_migrations()
        .await
        .expect("Failed to run migrations");

    let pool = db.pool();
    let repo = db.repository();

    // 1. Create Users
    // Alice: Platform Admin
    let user_alice = repo
        .create_user(CreateUserDto {
            username: "alice_admin".to_string(),
            email: "alice@apich.org".to_string(),
            password_hash: "hash_alice".to_string(),
            display_name: "Alice Platform Admin".to_string(),
            role: Some(UserRole::Admin),
            is_platform_admin: Some(true),
            storage_quota_bytes: None,
        })
        .await
        .expect("Failed to create user alice");

    // Bob: Org Owner for "Quantum Labs"
    let user_bob = repo
        .create_user(CreateUserDto {
            username: "bob_owner".to_string(),
            email: "bob@quantum.org".to_string(),
            password_hash: "hash_bob".to_string(),
            display_name: "Bob Lab Director".to_string(),
            role: Some(UserRole::Member),
            is_platform_admin: Some(false),
            storage_quota_bytes: None,
        })
        .await
        .expect("Failed to create user bob");

    // Carol: Team Admin of "Theory Team"
    let user_carol = repo
        .create_user(CreateUserDto {
            username: "carol_theory".to_string(),
            email: "carol@quantum.org".to_string(),
            password_hash: "hash_carol".to_string(),
            display_name: "Carol Theory Lead".to_string(),
            role: Some(UserRole::Member),
            is_platform_admin: Some(false),
            storage_quota_bytes: None,
        })
        .await
        .expect("Failed to create user carol");

    // Dave: Member of "Surface Codes" (leaf sub-team)
    let user_dave = repo
        .create_user(CreateUserDto {
            username: "dave_researcher".to_string(),
            email: "dave@quantum.org".to_string(),
            password_hash: "hash_dave".to_string(),
            display_name: "Dave Researcher".to_string(),
            role: Some(UserRole::Member),
            is_platform_admin: Some(false),
            storage_quota_bytes: None,
        })
        .await
        .expect("Failed to create user dave");

    // 2. Create Organization: Quantum Labs (Bob is owner)
    let org = repo
        .create_organization(
            user_bob.id,
            CreateOrganizationDto {
                slug: "quantum-labs".to_string(),
                name: "Quantum Labs Enterprise".to_string(),
                description: Some("Quantum computing research institute".to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("Failed to create organization");

    // Add Carol and Dave to org
    repo.add_org_member(org.id, user_carol.id, "member")
        .await
        .expect("Failed to add carol to org");
    repo.add_org_member(org.id, user_dave.id, "member")
        .await
        .expect("Failed to add dave to org");

    // 3. Create Nested Team Hierarchy
    // Level 1: Quantum Theory (Carol is admin)
    let team_theory = repo
        .create_team(
            user_carol.id,
            CreateTeamDto {
                org_id: org.id,
                parent_team_id: None,
                name: "Quantum Theory".to_string(),
                slug: "theory".to_string(),
                description: Some("Theoretical physics & error correction".to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("Failed to create root team");

    // Level 2: Quantum Error Correction (Child of Quantum Theory)
    let team_qec = repo
        .create_team(
            user_carol.id,
            CreateTeamDto {
                org_id: org.id,
                parent_team_id: Some(team_theory.id),
                name: "Error Correction Group".to_string(),
                slug: "qec".to_string(),
                description: Some("Fault-tolerant architecture".to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("Failed to create sub-team level 2");

    // Level 3: Surface Codes (Child of QEC, Dave is member)
    let team_surface = repo
        .create_team(
            user_carol.id,
            CreateTeamDto {
                org_id: org.id,
                parent_team_id: Some(team_qec.id),
                name: "Surface Code Protocols".to_string(),
                slug: "surface-codes".to_string(),
                description: Some("Planar & toric surface codes".to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("Failed to create sub-team level 3");

    repo.add_team_member(team_surface.id, user_dave.id, "member")
        .await
        .expect("Failed to add dave to surface code team");

    // Also demonstrate multi-node identity: Dave is ALSO an admin in another independent group
    let team_outreach = repo
        .create_team(
            user_dave.id,
            CreateTeamDto {
                org_id: org.id,
                parent_team_id: None,
                name: "Public Outreach".to_string(),
                slug: "outreach".to_string(),
                description: Some("Community & open science".to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("Failed to create outreach team");

    // Verify team tree structure
    let tree = repo
        .get_team_tree_for_org(org.id)
        .await
        .expect("Failed to get team tree");
    assert_eq!(tree.len(), 2); // Quantum Theory and Public Outreach are top-level roots
    let theory_node = tree.iter().find(|n| n.team.id == team_theory.id).unwrap();
    assert_eq!(theory_node.children.len(), 1); // Error Correction Group
    assert_eq!(theory_node.children[0].team.id, team_qec.id);
    assert_eq!(theory_node.children[0].children.len(), 1); // Surface Code Protocols
    assert_eq!(theory_node.children[0].children[0].team.id, team_surface.id);

    // 4. Test Hierarchical Permission Delegation Rules
    // Rule A: Platform Admin (Alice) has omnipotent management rights
    assert!(IdentityPermissionResolver::is_platform_admin(pool, user_alice.id).await.unwrap());
    assert!(IdentityPermissionResolver::can_manage_org(pool, user_alice.id, org.id).await.unwrap());
    assert!(IdentityPermissionResolver::can_manage_team(pool, user_alice.id, team_theory.id).await.unwrap());
    assert!(IdentityPermissionResolver::can_manage_team(pool, user_alice.id, team_surface.id).await.unwrap());

    // Rule B: Org Owner (Bob) can manage Org and ALL descendant teams at any depth
    assert!(IdentityPermissionResolver::can_manage_org(pool, user_bob.id, org.id).await.unwrap());
    assert!(IdentityPermissionResolver::can_manage_team(pool, user_bob.id, team_theory.id).await.unwrap());
    assert!(IdentityPermissionResolver::can_manage_team(pool, user_bob.id, team_qec.id).await.unwrap());
    assert!(IdentityPermissionResolver::can_manage_team(pool, user_bob.id, team_surface.id).await.unwrap());

    // Rule C: Team Admin (Carol) can manage her team and ALL descendant subteams
    assert!(!IdentityPermissionResolver::can_manage_org(pool, user_carol.id, org.id).await.unwrap());
    assert!(IdentityPermissionResolver::can_manage_team(pool, user_carol.id, team_theory.id).await.unwrap());
    assert!(IdentityPermissionResolver::can_manage_team(pool, user_carol.id, team_qec.id).await.unwrap());
    assert!(IdentityPermissionResolver::can_manage_team(pool, user_carol.id, team_surface.id).await.unwrap());
    // But Carol CANNOT manage Outreach team (sibling branch)
    assert!(!IdentityPermissionResolver::can_manage_team(pool, user_carol.id, team_outreach.id).await.unwrap());

    // Rule D: Member (Dave) can manage Outreach (where he is admin), but CANNOT manage parent teams
    assert!(IdentityPermissionResolver::can_manage_team(pool, user_dave.id, team_outreach.id).await.unwrap());
    assert!(!IdentityPermissionResolver::can_manage_team(pool, user_dave.id, team_surface.id).await.unwrap());
    assert!(!IdentityPermissionResolver::can_manage_team(pool, user_dave.id, team_theory.id).await.unwrap());

    // 5. Project Lifecycle & Access Resolution
    let proj = repo
        .create_project(CreateProjectDto {
            org_id: org.id,
            team_id: Some(team_surface.id),
            owner_id: user_dave.id,
            name: "Rotated Surface Simulator".to_string(),
            slug: "rotated-surface-sim".to_string(),
            description: Some("High-threshold simulation suite".to_string()),
            storage_path: "/workspace/projects/rotated-surface".to_string(),
            settings: None,
        })
        .await
        .expect("Failed to create project");

    // Dave owns the project -> can manage and access
    assert!(IdentityPermissionResolver::can_manage_project(pool, user_dave.id, proj.id).await.unwrap());
    assert!(IdentityPermissionResolver::can_access_project(pool, user_dave.id, proj.id).await.unwrap());

    // Carol is ancestor admin of Surface Codes team -> can manage and access project!
    assert!(IdentityPermissionResolver::can_manage_project(pool, user_carol.id, proj.id).await.unwrap());
    assert!(IdentityPermissionResolver::can_access_project(pool, user_carol.id, proj.id).await.unwrap());

    // Bob is Org Owner -> can manage and access project!
    assert!(IdentityPermissionResolver::can_manage_project(pool, user_bob.id, proj.id).await.unwrap());
    assert!(IdentityPermissionResolver::can_access_project(pool, user_bob.id, proj.id).await.unwrap());

    // 6. Project Container Sandbox Mapping (1 Project + 1 User)
    let container_name = format!("sbx-{}-{}", proj.slug, user_dave.username);
    let sandbox = repo
        .upsert_project_sandbox(proj.id, user_dave.id, &container_name, "running")
        .await
        .expect("Failed to upsert project sandbox");
    assert_eq!(sandbox.status, "running");
    assert!(sandbox.last_started_at.is_some());

    // Stop sandbox
    let sandbox_stopped = repo
        .upsert_project_sandbox(proj.id, user_dave.id, &container_name, "stopped")
        .await
        .expect("Failed to stop project sandbox");
    assert_eq!(sandbox_stopped.status, "stopped");
    assert!(sandbox_stopped.last_stopped_at.is_some());

    // 7. Sessions, Passkeys, Invitations, & SSO
    // Session token
    let session = repo
        .create_user_session(
            user_dave.id,
            "sample_token_hash_hex_12345",
            chrono::Utc::now() + chrono::Duration::days(7),
            Some("Mozilla/5.0"),
            Some("127.0.0.1"),
        )
        .await
        .expect("Failed to create session");
    assert_eq!(session.user_id, user_dave.id);

    let auth_user = repo
        .get_user_by_session_token_hash("sample_token_hash_hex_12345")
        .await
        .expect("Failed to query session")
        .expect("User session not found");
    assert_eq!(auth_user.id, user_dave.id);

    // Passkey (FIDO2 credential)
    let cred = repo
        .save_fido2_credential(
            user_dave.id,
            "cred_id_fido2_base64_abc",
            &[1, 2, 3, 4, 5],
            10,
            "Dave YubiKey 5C",
            None,
        )
        .await
        .expect("Failed to save FIDO2 credential");
    assert_eq!(cred.counter, 10);

    repo.update_fido2_counter("cred_id_fido2_base64_abc", 11)
        .await
        .expect("Failed to update counter");
    let updated_cred = repo
        .get_fido2_credential_by_id("cred_id_fido2_base64_abc")
        .await
        .expect("Failed to fetch cred")
        .unwrap();
    assert_eq!(updated_cred.counter, 11);

    // Invitation
    let invite = repo
        .create_invitation(
            "token_secret_invite_999",
            CreateInvitationDto {
                email: "new_researcher@lab.org".to_string(),
                org_id: Some(org.id),
                team_id: Some(team_surface.id),
                role: Some("member".to_string()),
                inviter_id: Some(user_carol.id),
                expires_at: chrono::Utc::now() + chrono::Duration::days(3),
            },
        )
        .await
        .expect("Failed to create invitation");
    assert_eq!(invite.email, "new_researcher@lab.org");

    repo.mark_invitation_used("token_secret_invite_999")
        .await
        .expect("Failed to mark invitation used");
    assert!(repo
        .get_invitation_by_token("token_secret_invite_999")
        .await
        .unwrap()
        .is_none());

    // OAuth2 SSO
    let client = repo
        .create_oauth_client(CreateOAuthClientDto {
            client_id: "jupyterlab-hub".to_string(),
            client_secret_hash: Some("secret_hash_oauth".to_string()),
            name: "JupyterLab Cluster Hub".to_string(),
            redirect_uris: vec!["https://jupyter.quantum.org/oauth/callback".to_string()],
            is_confidential: true,
        })
        .await
        .expect("Failed to create oauth client");
    assert_eq!(client.client_id, "jupyterlab-hub");

    let auth_code = repo
        .create_oauth_auth_code(
            "code_auth_xyz_123",
            "jupyterlab-hub",
            user_dave.id,
            "https://jupyter.quantum.org/oauth/callback",
            "openid profile email",
            chrono::Utc::now() + chrono::Duration::minutes(5),
        )
        .await
        .expect("Failed to create oauth code");
    assert_eq!(auth_code.code, "code_auth_xyz_123");

    let consumed = repo
        .consume_oauth_auth_code("code_auth_xyz_123")
        .await
        .expect("Failed to consume code")
        .expect("Code should exist");
    assert_eq!(consumed.user_id, user_dave.id);

    // Second consumption must fail (single-use code)
    let second_consume = repo
        .consume_oauth_auth_code("code_auth_xyz_123")
        .await
        .expect("Query failed");
    assert!(second_consume.is_none());

    // Cleanup container
    let _ = container.stop(10).await;
}
