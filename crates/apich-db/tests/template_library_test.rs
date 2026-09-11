mod common;

use apich_db::CreateOrganizationDto;
use apich_db::CreateTemplateDto;
use apich_db::CreateUserDto;
use apich_db::Database;
use apich_db::IdentityPermissionResolver;
use apich_db::PostgresConfig;
use apich_db::PostgresContainer;
use apich_db::PublishTemplateVersionDto;
use apich_db::UserRole;
use common::test_temp_dir;
use std::time::Duration;

/// Real Postgres, real migrations, real repo methods -- covers the three visibility modes a
/// published template can have (private/shared/public) end to end, including that "shared" means
/// specific orgs the owner chose, not necessarily their own.
#[tokio::test]
async fn test_template_library_visibility_and_versions() {
    let temp_data = test_temp_dir();
    let temp_backup = test_temp_dir();

    let config = PostgresConfig::builder(temp_data.path(), temp_backup.path())
        .container_name("apich-pg-templates-test")
        .host_port(5441)
        .database("test_templates_db")
        .admin_user("postgres")
        .admin_password("admin_templates_pwd")
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
    db.run_migrations().await.expect("Failed to run migrations");

    let pool = db.pool();
    let repo = db.repository();

    let make_user = |username: &str| {
        CreateUserDto {
            username: username.to_string(),
            email: format!("{username}@example.com"),
            password_hash: "hash".to_string(),
            display_name: username.to_string(),
            role: Some(UserRole::Member),
            is_platform_admin: Some(false),
            storage_quota_bytes: None,
        }
    };

    let owner = repo
        .create_user(make_user("template_owner"))
        .await
        .expect("create owner");
    let outsider = repo
        .create_user(make_user("outsider"))
        .await
        .expect("create outsider");
    let org_member = repo
        .create_user(make_user("org_member"))
        .await
        .expect("create org member");

    // A template is published as 'private' by default -- only the owner can see it.
    let template = repo
        .create_template(CreateTemplateDto {
            kind: "kanban".to_string(),
            name: "Research Sprint Board".to_string(),
            slug: "research-sprint-board".to_string(),
            description: Some("Backlog / In Review / Shipped".to_string()),
            owner_user_id: owner.id,
            visibility: "private".to_string(),
        })
        .await
        .expect("create template");

    assert!(
        IdentityPermissionResolver::can_access_template(pool, owner.id, template.id)
            .await
            .unwrap()
    );
    assert!(
        !IdentityPermissionResolver::can_access_template(pool, outsider.id, template.id)
            .await
            .unwrap()
    );
    assert!(
        IdentityPermissionResolver::can_manage_template(pool, owner.id, template.id)
            .await
            .unwrap()
    );
    assert!(
        !IdentityPermissionResolver::can_manage_template(pool, outsider.id, template.id)
            .await
            .unwrap()
    );

    // Publish a version with real kanban-preset content.
    let version1 = repo
        .publish_template_version(PublishTemplateVersionDto {
            template_id: template.id,
            version_label: "1.0.0".to_string(),
            changelog: Some("Initial layout".to_string()),
            content: serde_json::json!({
                "columns": [
                    {"id": "backlog", "title": "Backlog", "is_done": false},
                    {"id": "review", "title": "In Review", "is_done": false},
                    {"id": "shipped", "title": "Shipped", "is_done": true},
                ]
            }),
            published_by: owner.id,
        })
        .await
        .expect("publish version 1");
    assert_eq!(version1.version_label, "1.0.0");

    let latest = repo
        .get_latest_template_version(template.id)
        .await
        .unwrap()
        .expect("latest version");
    assert_eq!(latest.id, version1.id);

    // A second version distinguishable from the first, listed newest-first.
    let version2 = repo
        .publish_template_version(PublishTemplateVersionDto {
            template_id: template.id,
            version_label: "1.1.0".to_string(),
            changelog: Some("Added a Blocked column".to_string()),
            content: serde_json::json!({"columns": [
                {"id": "backlog", "title": "Backlog", "is_done": false},
                {"id": "blocked", "title": "Blocked", "is_done": false},
                {"id": "review", "title": "In Review", "is_done": false},
                {"id": "shipped", "title": "Shipped", "is_done": true},
            ]}),
            published_by: owner.id,
        })
        .await
        .expect("publish version 2");

    let versions = repo.list_template_versions(template.id).await.unwrap();
    assert_eq!(versions.len(), 2);
    assert_eq!(
        versions[0].id, version2.id,
        "newest version must sort first"
    );
    assert_eq!(versions[1].id, version1.id);

    // Duplicate version labels on the same template must be rejected (uq_template_version_label).
    let dup = repo
        .publish_template_version(PublishTemplateVersionDto {
            template_id: template.id,
            version_label: "1.1.0".to_string(),
            changelog: None,
            content: serde_json::json!({"columns": []}),
            published_by: owner.id,
        })
        .await;
    assert!(
        dup.is_err(),
        "publishing the same version_label twice must fail"
    );

    // Owned-private templates show up in the owner's own gallery, nobody else's.
    let owner_visible = repo.list_visible_templates(owner.id, None).await.unwrap();
    assert!(owner_visible.iter().any(|t| t.id == template.id));
    let outsider_visible = repo
        .list_visible_templates(outsider.id, None)
        .await
        .unwrap();
    assert!(!outsider_visible.iter().any(|t| t.id == template.id));

    // Share it to an org the OWNER doesn't belong to -- proving sharing isn't limited to the
    // publisher's own org membership. `org_member` (a member of that org) gains access;
    // `outsider` (a member of neither) still doesn't.
    let shared_org = repo
        .create_organization(
            org_member.id,
            CreateOrganizationDto {
                slug: "partner-lab".to_string(),
                name: "Partner Lab".to_string(),
                ..Default::default()
            },
        )
        .await
        .expect("create shared org");

    repo.update_template_visibility(template.id, "shared")
        .await
        .unwrap();
    assert!(
        !IdentityPermissionResolver::can_access_template(pool, org_member.id, template.id)
            .await
            .unwrap(),
        "not shared yet"
    );

    repo.add_template_share(template.id, Some(shared_org.id), None)
        .await
        .expect("add share");
    assert!(
        IdentityPermissionResolver::can_access_template(pool, org_member.id, template.id)
            .await
            .unwrap()
    );
    assert!(
        !IdentityPermissionResolver::can_access_template(pool, outsider.id, template.id)
            .await
            .unwrap(),
        "outsider is in no shared org"
    );
    // The owner keeps access regardless of sharing (owner-check short-circuits visibility).
    assert!(
        IdentityPermissionResolver::can_access_template(pool, owner.id, template.id)
            .await
            .unwrap()
    );

    let org_member_visible = repo
        .list_visible_templates(org_member.id, None)
        .await
        .unwrap();
    assert!(org_member_visible.iter().any(|t| t.id == template.id));

    // Removing the share revokes access again.
    let shares = repo.list_template_shares(template.id).await.unwrap();
    assert_eq!(shares.len(), 1);
    repo.remove_template_share(shares[0].id).await.unwrap();
    assert!(
        !IdentityPermissionResolver::can_access_template(pool, org_member.id, template.id)
            .await
            .unwrap()
    );

    // Public: everyone, no share entries needed.
    repo.update_template_visibility(template.id, "public")
        .await
        .unwrap();
    assert!(
        IdentityPermissionResolver::can_access_template(pool, outsider.id, template.id)
            .await
            .unwrap()
    );
    let outsider_visible_now = repo
        .list_visible_templates(outsider.id, Some("kanban"))
        .await
        .unwrap();
    assert!(outsider_visible_now.iter().any(|t| t.id == template.id));
    let outsider_visible_wrong_kind = repo
        .list_visible_templates(outsider.id, Some("note"))
        .await
        .unwrap();
    assert!(
        !outsider_visible_wrong_kind
            .iter()
            .any(|t| t.id == template.id),
        "kind filter must exclude non-matching templates"
    );

    // Deleting the template cascades away its versions and shares (FK ON DELETE CASCADE).
    repo.delete_template(template.id).await.unwrap();
    assert!(repo
        .get_template_by_id(template.id)
        .await
        .unwrap()
        .is_none());
    assert!(repo
        .list_template_versions(template.id)
        .await
        .unwrap()
        .is_empty());
}
