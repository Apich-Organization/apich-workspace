use crate::error::Result;
use crate::models::{
    AuditLog, CreateAuditLogDto, CreateDocumentDto, CreateInvitationDto, CreateKnowledgeEdgeDto,
    CreateKnowledgeNodeDto, CreateOAuthClientDto, CreateOrganizationDto, CreateProjectDto,
    CreateTeamDto, CreateUserDto, CreateWorkspaceDto, Document, DocumentSearchResult,
    Fido2Credential, Invitation, KnowledgeEdge, KnowledgeNode, MemberRole, OAuthAuthCode,
    OAuthClient, OrgMemberWithUser, Organization, Project, ProjectMember, ProjectMemberWithUser,
    ProjectSandbox, SystemSettings, Team, TeamMemberWithUser, TeamTreeNode,
    UpdateOrganizationDto, UpdateSystemSettingsDto, UpdateTeamDto, UpdateUserProfileDto, User,
    UserRole, UserSession, Workspace, WorkspaceMember,

};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

pub struct Repository<'a> {
    pool: &'a PgPool,
}

impl<'a> Repository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    // --- User Operations ---

    pub async fn create_user(&self, dto: CreateUserDto) -> Result<User> {
        let id = Uuid::now_v7();
        let role = dto.role.unwrap_or_default();
        let is_platform_admin = dto.is_platform_admin.unwrap_or(matches!(role, UserRole::Admin));
        let quota = dto.storage_quota_bytes.unwrap_or(10 * 1024 * 1024 * 1024);

        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (id, username, email, password_hash, display_name, role, is_platform_admin, storage_quota_bytes)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(dto.username)
        .bind(dto.email)
        .bind(dto.password_hash)
        .bind(dto.display_name)
        .bind(role)
        .bind(is_platform_admin)
        .bind(quota)
        .fetch_one(self.pool)
        .await?;

        Ok(user)
    }

    pub async fn get_user_by_id(&self, id: Uuid) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool)
            .await?;
        Ok(user)
    }

    pub async fn get_user_by_username(&self, username: &str) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
            .bind(username)
            .fetch_optional(self.pool)
            .await?;
        Ok(user)
    }

    pub async fn update_user_profile(&self, id: Uuid, dto: UpdateUserProfileDto) -> Result<User> {
        let user = sqlx::query_as::<_, User>(
            r#"
            UPDATE users
            SET display_name = COALESCE($2, display_name),
                email = COALESCE($3, email),
                avatar_url = COALESCE($4, avatar_url),
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(dto.display_name)
        .bind(dto.email)
        .bind(dto.avatar_url)
        .fetch_one(self.pool)
        .await?;

        Ok(user)
    }

    pub async fn update_user_password(&self, id: Uuid, password_hash: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE users
            SET password_hash = $2,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(password_hash)
        .execute(self.pool)
        .await?;

        Ok(())
    }


    // --- Workspace Operations ---

    pub async fn create_workspace(&self, dto: CreateWorkspaceDto) -> Result<Workspace> {
        let id = dto.id.unwrap_or_else(Uuid::now_v7);
        let visibility = dto.visibility.unwrap_or_default();
        let container_name = format!("apich-ws-{}", dto.slug);
        let settings = dto.settings.unwrap_or_else(|| serde_json::json!({}));

        let mut tx = self.pool.begin().await?;

        let ws = sqlx::query_as::<_, Workspace>(
            r#"
            INSERT INTO workspaces (id, owner_id, slug, name, description, visibility, container_name, settings)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(dto.owner_id)
        .bind(dto.slug)
        .bind(dto.name)
        .bind(dto.description)
        .bind(visibility)
        .bind(container_name)
        .bind(settings)
        .fetch_one(&mut *tx)
        .await?;

        // Add owner as workspace member
        sqlx::query(
            r#"
            INSERT INTO workspace_members (workspace_id, user_id, role)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(ws.id)
        .bind(dto.owner_id)
        .bind(MemberRole::Owner)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(ws)
    }

    pub async fn get_workspace_by_id(&self, id: Uuid) -> Result<Option<Workspace>> {
        let ws = sqlx::query_as::<_, Workspace>("SELECT * FROM workspaces WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool)
            .await?;
        Ok(ws)
    }

    pub async fn add_workspace_member(
        &self,
        workspace_id: Uuid,
        user_id: Uuid,
        role: MemberRole,
    ) -> Result<WorkspaceMember> {
        let member = sqlx::query_as::<_, WorkspaceMember>(
            r#"
            INSERT INTO workspace_members (workspace_id, user_id, role)
            VALUES ($1, $2, $3)
            ON CONFLICT (workspace_id, user_id) DO UPDATE SET role = EXCLUDED.role
            RETURNING *
            "#,
        )
        .bind(workspace_id)
        .bind(user_id)
        .bind(role)
        .fetch_one(self.pool)
        .await?;

        Ok(member)
    }

    // --- Document Operations ---

    pub async fn create_document(&self, dto: CreateDocumentDto) -> Result<Document> {
        let id = dto.id.unwrap_or_else(Uuid::now_v7);
        let content = dto.content.unwrap_or_default();
        let doc_type = dto.doc_type.unwrap_or_default();
        let metadata = dto.metadata.unwrap_or_else(|| serde_json::json!({}));

        let doc = sqlx::query_as::<_, Document>(
            r#"
            INSERT INTO documents (id, workspace_id, rel_path, title, content, doc_type, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(dto.workspace_id)
        .bind(dto.rel_path)
        .bind(dto.title)
        .bind(content)
        .bind(doc_type)
        .bind(metadata)
        .fetch_one(self.pool)
        .await?;

        Ok(doc)
    }

    /// Atomic document upsert leveraging PostgreSQL MERGE syntax
    pub async fn upsert_document(&self, dto: CreateDocumentDto) -> Result<Document> {
        let id = dto.id.unwrap_or_else(Uuid::now_v7);
        let content = dto.content.unwrap_or_default();
        let doc_type = dto.doc_type.unwrap_or_default();
        let metadata = dto.metadata.unwrap_or_else(|| serde_json::json!({}));

        let doc = sqlx::query_as::<_, Document>(
            r#"
            INSERT INTO documents (id, workspace_id, rel_path, title, content, doc_type, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (workspace_id, rel_path) DO UPDATE SET
                title = EXCLUDED.title,
                content = EXCLUDED.content,
                doc_type = EXCLUDED.doc_type,
                version = documents.version + 1,
                metadata = EXCLUDED.metadata,
                updated_at = CURRENT_TIMESTAMP
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(dto.workspace_id)
        .bind(dto.rel_path)
        .bind(dto.title)
        .bind(content)
        .bind(doc_type)
        .bind(metadata)
        .fetch_one(self.pool)
        .await?;

        Ok(doc)
    }

    pub async fn get_document_by_path(
        &self,
        workspace_id: Uuid,
        rel_path: &str,
    ) -> Result<Option<Document>> {
        let doc = sqlx::query_as::<_, Document>(
            "SELECT * FROM documents WHERE workspace_id = $1 AND rel_path = $2",
        )
        .bind(workspace_id)
        .bind(rel_path)
        .fetch_optional(self.pool)
        .await?;
        Ok(doc)
    }

    /// Full-text search on documents using PostgreSQL 18 generated tsvector and GIN index
    pub async fn search_documents_fulltext(
        &self,
        workspace_id: Uuid,
        query: &str,
        limit: i64,
    ) -> Result<Vec<DocumentSearchResult>> {
        let results = sqlx::query_as::<_, DocumentSearchResult>(
            r#"
            SELECT id, workspace_id, rel_path, title, content, doc_type, version, metadata, created_at, updated_at,
                   ts_rank(search_tokens, websearch_to_tsquery('english', $2)) AS rank
            FROM documents
            WHERE workspace_id = $1 AND search_tokens @@ websearch_to_tsquery('english', $2)
            ORDER BY rank DESC, updated_at DESC
            LIMIT $3;
            "#,
        )
        .bind(workspace_id)
        .bind(query)
        .bind(limit)
        .fetch_all(self.pool)
        .await?;

        Ok(results)
    }

    /// Query documents matching JSONB metadata containment (`@>`), accelerated by GIN jsonb_path_ops
    pub async fn find_documents_by_metadata(
        &self,
        workspace_id: Uuid,
        filter: &serde_json::Value,
    ) -> Result<Vec<Document>> {
        let docs = sqlx::query_as::<_, Document>(
            r#"
            SELECT * FROM documents
            WHERE workspace_id = $1 AND metadata @> $2
            ORDER BY updated_at DESC;
            "#,
        )
        .bind(workspace_id)
        .bind(filter)
        .fetch_all(self.pool)
        .await?;

        Ok(docs)
    }

    /// Extract array elements from JSONB metadata using standard SQL/JSON `json_table` (PostgreSQL 17/18)
    pub async fn extract_document_tags(
        &self,
        doc_id: Uuid,
    ) -> Result<Vec<String>> {
        let tags: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT jt.tag
            FROM documents d,
            JSON_TABLE(
                d.metadata,
                '$.tags[*]'
                COLUMNS (
                    tag TEXT PATH '$'
                )
            ) AS jt
            WHERE d.id = $1;
            "#,
        )
        .bind(doc_id)
        .fetch_all(self.pool)
        .await?;

        Ok(tags)
    }

    /// Extract creation timestamp from UUIDv7 natively via PostgreSQL 18 function `uuid_extract_timestamp`
    pub async fn extract_uuidv7_timestamp(&self, id: Uuid) -> Result<DateTime<Utc>> {
        let ts: DateTime<Utc> = sqlx::query_scalar(
            "SELECT uuid_extract_timestamp($1);"
        )
        .bind(id)
        .fetch_one(self.pool)
        .await?;

        Ok(ts)
    }

    // --- Knowledge Graph Operations (Plan.md Section 4) ---

    pub async fn create_knowledge_node(
        &self,
        dto: CreateKnowledgeNodeDto,
    ) -> Result<KnowledgeNode> {
        let id = dto.id.unwrap_or_else(Uuid::now_v7);
        let metadata = dto.metadata.unwrap_or_else(|| serde_json::json!({}));

        let node = sqlx::query_as::<_, KnowledgeNode>(
            r#"
            INSERT INTO knowledge_nodes (id, workspace_id, node_type, title, content_hash, metadata)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(dto.workspace_id)
        .bind(dto.node_type)
        .bind(dto.title)
        .bind(dto.content_hash)
        .bind(metadata)
        .fetch_one(self.pool)
        .await?;

        Ok(node)
    }

    pub async fn create_knowledge_edge(
        &self,
        dto: CreateKnowledgeEdgeDto,
    ) -> Result<KnowledgeEdge> {
        let id = dto.id.unwrap_or_else(Uuid::now_v7);
        let weight = dto.weight.unwrap_or(1.0);

        let edge = sqlx::query_as::<_, KnowledgeEdge>(
            r#"
            INSERT INTO knowledge_edges (id, workspace_id, source_id, target_id, relation_type, weight)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(dto.workspace_id)
        .bind(dto.source_id)
        .bind(dto.target_id)
        .bind(dto.relation_type)
        .bind(weight)
        .fetch_one(self.pool)
        .await?;

        Ok(edge)
    }

    // --- Audit Log Operations (Plan.md Section 6) ---

    pub async fn record_audit_log(&self, dto: CreateAuditLogDto) -> Result<AuditLog> {
        let id = Uuid::now_v7();

        let log = sqlx::query_as::<_, AuditLog>(
            r#"
            INSERT INTO audit_logs (id, user_id, workspace_id, action, details, ip_address)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(dto.user_id)
        .bind(dto.workspace_id)
        .bind(dto.action)
        .bind(dto.details)
        .bind(dto.ip_address)
        .fetch_one(self.pool)
        .await?;

        Ok(log)
    }

    // --- User Extensions ---

    pub async fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
            .bind(email)
            .fetch_optional(self.pool)
            .await?;
        Ok(user)
    }

    pub async fn list_users(&self) -> Result<Vec<User>> {
        let users = sqlx::query_as::<_, User>("SELECT * FROM users ORDER BY created_at DESC")
            .fetch_all(self.pool)
            .await?;
        Ok(users)
    }

    // --- Organization Operations ---

    pub async fn create_organization(
        &self,
        owner_id: Uuid,
        dto: CreateOrganizationDto,
    ) -> Result<Organization> {
        let id = Uuid::now_v7();
        let mut tx = self.pool.begin().await?;

        let org = sqlx::query_as::<_, Organization>(
            r#"
            INSERT INTO organizations (id, slug, name, description)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&dto.slug)
        .bind(&dto.name)
        .bind(&dto.description)
        .fetch_one(&mut *tx)
        .await?;

        // Add creator as organization owner
        sqlx::query(
            r#"
            INSERT INTO org_members (org_id, user_id, role)
            VALUES ($1, $2, 'owner')
            "#,
        )
        .bind(org.id)
        .bind(owner_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(org)
    }

    pub async fn get_organization_by_id(&self, id: Uuid) -> Result<Option<Organization>> {
        let org = sqlx::query_as::<_, Organization>("SELECT * FROM organizations WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool)
            .await?;
        Ok(org)
    }

    pub async fn get_organization_by_slug(&self, slug: &str) -> Result<Option<Organization>> {
        let org = sqlx::query_as::<_, Organization>("SELECT * FROM organizations WHERE slug = $1")
            .bind(slug)
            .fetch_optional(self.pool)
            .await?;
        Ok(org)
    }

    pub async fn list_organizations_for_user(&self, user_id: Uuid) -> Result<Vec<Organization>> {
        let orgs = sqlx::query_as::<_, Organization>(
            r#"
            SELECT o.* FROM organizations o
            JOIN org_members om ON o.id = om.org_id
            WHERE om.user_id = $1
            ORDER BY o.name ASC
            "#,
        )
        .bind(user_id)
        .fetch_all(self.pool)
        .await?;
        Ok(orgs)
    }

    pub async fn list_organizations(&self, limit: i64) -> Result<Vec<Organization>> {
        let orgs = sqlx::query_as::<_, Organization>(
            "SELECT * FROM organizations ORDER BY name ASC LIMIT $1",
        )
        .bind(limit)
        .fetch_all(self.pool)
        .await?;
        Ok(orgs)
    }

    pub async fn add_org_member(&self, org_id: Uuid, user_id: Uuid, role: &str) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO org_members (org_id, user_id, role)
            VALUES ($1, $2, $3)
            ON CONFLICT (org_id, user_id) DO UPDATE SET role = EXCLUDED.role
            "#,
        )
        .bind(org_id)
        .bind(user_id)
        .bind(role)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    pub async fn remove_org_member(&self, org_id: Uuid, user_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM org_members WHERE org_id = $1 AND user_id = $2")
            .bind(org_id)
            .bind(user_id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    pub async fn list_org_members(&self, org_id: Uuid) -> Result<Vec<OrgMemberWithUser>> {
        let rows = sqlx::query_as::<_, (Uuid, Uuid, String, String, String, String, Option<String>, DateTime<Utc>)>(
            r#"
            SELECT om.org_id, om.user_id, om.role, u.username, u.email, u.display_name, u.avatar_url, om.joined_at
            FROM org_members om
            JOIN users u ON om.user_id = u.id
            WHERE om.org_id = $1
            ORDER BY om.joined_at ASC
            "#,
        )
        .bind(org_id)
        .fetch_all(self.pool)
        .await?;

        let members = rows
            .into_iter()
            .map(|(org_id, user_id, role, username, email, display_name, avatar_url, joined_at)| {
                OrgMemberWithUser {
                    org_id,
                    user_id,
                    role,
                    username,
                    email,
                    display_name,
                    avatar_url,
                    joined_at,
                }
            })
            .collect();

        Ok(members)
    }

    pub async fn update_organization(
        &self,
        id: Uuid,
        dto: UpdateOrganizationDto,
    ) -> Result<Organization> {
        let org = sqlx::query_as::<_, Organization>(
            r#"
            UPDATE organizations
            SET name = COALESCE($2, name),
                slug = COALESCE($3, slug),
                description = COALESCE($4, description),
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(dto.name)
        .bind(dto.slug)
        .bind(dto.description)
        .fetch_one(self.pool)
        .await?;

        Ok(org)
    }

    pub async fn delete_organization(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM organizations WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    // --- Team Operations (Recursive tree) ---


    pub async fn create_team(&self, creator_id: Uuid, dto: CreateTeamDto) -> Result<Team> {
        let id = Uuid::now_v7();
        let mut tx = self.pool.begin().await?;

        let team = sqlx::query_as::<_, Team>(
            r#"
            INSERT INTO teams (id, org_id, parent_team_id, name, slug, description)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(dto.org_id)
        .bind(dto.parent_team_id)
        .bind(&dto.name)
        .bind(&dto.slug)
        .bind(&dto.description)
        .fetch_one(&mut *tx)
        .await?;

        // Creator becomes team admin
        sqlx::query(
            r#"
            INSERT INTO team_members (team_id, user_id, role)
            VALUES ($1, $2, 'admin')
            ON CONFLICT (team_id, user_id) DO UPDATE SET role = 'admin'
            "#,
        )
        .bind(team.id)
        .bind(creator_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(team)
    }

    pub async fn get_team_by_id(&self, id: Uuid) -> Result<Option<Team>> {
        let team = sqlx::query_as::<_, Team>("SELECT * FROM teams WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool)
            .await?;
        Ok(team)
    }

    pub async fn list_teams_by_org(&self, org_id: Uuid) -> Result<Vec<Team>> {
        let teams = sqlx::query_as::<_, Team>(
            "SELECT * FROM teams WHERE org_id = $1 ORDER BY parent_team_id NULLS FIRST, name ASC",
        )
        .bind(org_id)
        .fetch_all(self.pool)
        .await?;
        Ok(teams)
    }

    /// Build hierarchical team tree for an organization
    pub async fn get_team_tree_for_org(&self, org_id: Uuid) -> Result<Vec<TeamTreeNode>> {
        let all_teams = self.list_teams_by_org(org_id).await?;

        // Helper to recursively assemble tree nodes
        fn assemble_subtrees(
            parent_id: Option<Uuid>,
            teams: &[Team],
        ) -> Vec<TeamTreeNode> {
            teams
                .iter()
                .filter(|t| t.parent_team_id == parent_id)
                .map(|t| {
                    let children = assemble_subtrees(Some(t.id), teams);
                    TeamTreeNode {
                        team: t.clone(),
                        children,
                        members_count: 0,
                    }
                })
                .collect()
        }

        Ok(assemble_subtrees(None, &all_teams))
    }

    pub async fn add_team_member(&self, team_id: Uuid, user_id: Uuid, role: &str) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO team_members (team_id, user_id, role)
            VALUES ($1, $2, $3)
            ON CONFLICT (team_id, user_id) DO UPDATE SET role = EXCLUDED.role
            "#,
        )
        .bind(team_id)
        .bind(user_id)
        .bind(role)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    pub async fn remove_team_member(&self, team_id: Uuid, user_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM team_members WHERE team_id = $1 AND user_id = $2")
            .bind(team_id)
            .bind(user_id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    pub async fn list_team_members(&self, team_id: Uuid) -> Result<Vec<TeamMemberWithUser>> {
        let rows = sqlx::query_as::<_, (Uuid, Uuid, String, String, String, String, Option<String>, DateTime<Utc>)>(
            r#"
            SELECT tm.team_id, tm.user_id, tm.role, u.username, u.email, u.display_name, u.avatar_url, tm.joined_at
            FROM team_members tm
            JOIN users u ON tm.user_id = u.id
            WHERE tm.team_id = $1
            ORDER BY tm.joined_at ASC
            "#,
        )
        .bind(team_id)
        .fetch_all(self.pool)
        .await?;

        let members = rows
            .into_iter()
            .map(|(team_id, user_id, role, username, email, display_name, avatar_url, joined_at)| {
                TeamMemberWithUser {
                    team_id,
                    user_id,
                    role,
                    username,
                    email,
                    display_name,
                    avatar_url,
                    joined_at,
                }
            })
            .collect();

        Ok(members)
    }

    pub async fn update_team(&self, id: Uuid, dto: UpdateTeamDto) -> Result<Team> {
        let (update_parent, parent_id) = match dto.parent_team_id {
            Some(p) => (true, p),
            None => (false, None),
        };

        let team = sqlx::query_as::<_, Team>(
            r#"
            UPDATE teams
            SET name = COALESCE($2, name),
                slug = COALESCE($3, slug),
                description = COALESCE($4, description),
                parent_team_id = CASE WHEN $5 = true THEN $6 ELSE parent_team_id END,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(dto.name)
        .bind(dto.slug)
        .bind(dto.description)
        .bind(update_parent)
        .bind(parent_id)
        .fetch_one(self.pool)
        .await?;

        Ok(team)
    }

    pub async fn delete_team(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM teams WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    // --- Project Operations (Core Unit) ---


    pub async fn create_project(&self, dto: CreateProjectDto) -> Result<Project> {
        let id = Uuid::now_v7();
        let settings = dto.settings.unwrap_or_else(|| serde_json::json!({}));

        let proj = sqlx::query_as::<_, Project>(
            r#"
            INSERT INTO projects (id, org_id, team_id, owner_id, name, slug, description, storage_path, settings)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(dto.org_id)
        .bind(dto.team_id)
        .bind(dto.owner_id)
        .bind(&dto.name)
        .bind(&dto.slug)
        .bind(&dto.description)
        .bind(&dto.storage_path)
        .bind(settings)
        .fetch_one(self.pool)
        .await?;

        // Automatically insert owner as project owner member
        let _ = sqlx::query(
            "INSERT INTO project_members (project_id, user_id, role) VALUES ($1, $2, 'owner') ON CONFLICT (project_id, user_id) DO NOTHING",
        )
        .bind(id)
        .bind(dto.owner_id)
        .execute(self.pool)
        .await;

        Ok(proj)
    }

    pub async fn get_project_by_id(&self, id: Uuid) -> Result<Option<Project>> {
        let proj = sqlx::query_as::<_, Project>("SELECT * FROM projects WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool)
            .await?;
        Ok(proj)
    }

    pub async fn get_project_by_slug(&self, org_id: Uuid, slug: &str) -> Result<Option<Project>> {
        let proj = sqlx::query_as::<_, Project>("SELECT * FROM projects WHERE org_id = $1 AND slug = $2")
            .bind(org_id)
            .bind(slug)
            .fetch_optional(self.pool)
            .await?;
        Ok(proj)
    }

    pub async fn list_projects_for_user(&self, user_id: Uuid) -> Result<Vec<Project>> {
        let projs = sqlx::query_as::<_, Project>(
            r#"
            SELECT DISTINCT p.* FROM projects p
            LEFT JOIN project_members pm ON p.id = pm.project_id AND pm.user_id = $1
            LEFT JOIN team_members tm ON p.team_id = tm.team_id AND tm.user_id = $1
            LEFT JOIN org_members om ON p.org_id = om.org_id AND om.user_id = $1
            WHERE p.status != 'deleted' AND (
                p.owner_id = $1
                OR pm.user_id = $1
                OR tm.user_id = $1
                OR om.user_id = $1
            )
            ORDER BY p.updated_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(self.pool)
        .await?;
        Ok(projs)
    }

    pub async fn list_all_projects_admin(&self) -> Result<Vec<Project>> {
        let projs = sqlx::query_as::<_, Project>(
            "SELECT * FROM projects WHERE status != 'deleted' ORDER BY updated_at DESC",
        )
        .fetch_all(self.pool)
        .await?;
        Ok(projs)
    }

    pub async fn update_project_status(&self, id: Uuid, status: &str) -> Result<()> {
        sqlx::query("UPDATE projects SET status = $1 WHERE id = $2")
            .bind(status)
            .bind(id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    pub async fn set_project_vcs_initialized(&self, id: Uuid, initialized: bool) -> Result<()> {
        sqlx::query("UPDATE projects SET vcs_initialized = $1 WHERE id = $2")
            .bind(initialized)
            .bind(id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    // --- Project Members & Sharing ---

    pub async fn add_project_member(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        role: &str,
    ) -> Result<ProjectMember> {
        let member = sqlx::query_as::<_, ProjectMember>(
            r#"
            INSERT INTO project_members (project_id, user_id, role)
            VALUES ($1, $2, $3)
            ON CONFLICT (project_id, user_id) DO UPDATE SET role = EXCLUDED.role, updated_at = CURRENT_TIMESTAMP
            RETURNING *
            "#,
        )
        .bind(project_id)
        .bind(user_id)
        .bind(role)
        .fetch_one(self.pool)
        .await?;

        Ok(member)
    }

    pub async fn update_project_member_role(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        role: &str,
    ) -> Result<ProjectMember> {
        let member = sqlx::query_as::<_, ProjectMember>(
            r#"
            UPDATE project_members
            SET role = $1, updated_at = CURRENT_TIMESTAMP
            WHERE project_id = $2 AND user_id = $3
            RETURNING *
            "#,
        )
        .bind(role)
        .bind(project_id)
        .bind(user_id)
        .fetch_one(self.pool)
        .await?;

        Ok(member)
    }

    pub async fn remove_project_member(
        &self,
        project_id: Uuid,
        user_id: Uuid,
    ) -> Result<()> {
        sqlx::query("DELETE FROM project_members WHERE project_id = $1 AND user_id = $2")
            .bind(project_id)
            .bind(user_id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    pub async fn list_project_members(
        &self,
        project_id: Uuid,
    ) -> Result<Vec<ProjectMemberWithUser>> {
        let members = sqlx::query_as::<_, ProjectMemberWithUser>(
            r#"
            SELECT
                pm.project_id,
                pm.user_id,
                pm.role,
                u.username,
                u.email,
                u.display_name,
                pm.created_at
            FROM project_members pm
            JOIN users u ON pm.user_id = u.id
            WHERE pm.project_id = $1
            ORDER BY
                CASE pm.role
                    WHEN 'owner' THEN 1
                    WHEN 'admin' THEN 2
                    WHEN 'editor' THEN 3
                    WHEN 'viewer' THEN 4
                    ELSE 5
                END,
                pm.created_at ASC
            "#,
        )
        .bind(project_id)
        .fetch_all(self.pool)
        .await?;

        Ok(members)
    }

    pub async fn get_project_member(
        &self,
        project_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<ProjectMember>> {
        let member = sqlx::query_as::<_, ProjectMember>(
            "SELECT * FROM project_members WHERE project_id = $1 AND user_id = $2",
        )
        .bind(project_id)
        .bind(user_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(member)
    }

    // --- Project Sandbox Container Mapping (1 Project + 1 User) ---

    pub async fn upsert_project_sandbox(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        container_name: &str,
        status: &str,
    ) -> Result<ProjectSandbox> {
        let id = Uuid::now_v7();
        let is_running = status == "running";

        let sandbox = sqlx::query_as::<_, ProjectSandbox>(
            r#"
            INSERT INTO project_sandboxes (id, project_id, user_id, container_name, status, last_started_at)
            VALUES ($1, $2, $3, $4, $5, CASE WHEN $6 THEN CURRENT_TIMESTAMP ELSE NULL END)
            ON CONFLICT (project_id, user_id) DO UPDATE
            SET container_name = EXCLUDED.container_name,
                status = EXCLUDED.status,
                last_started_at = CASE WHEN $6 THEN CURRENT_TIMESTAMP ELSE project_sandboxes.last_started_at END,
                last_stopped_at = CASE WHEN NOT $6 THEN CURRENT_TIMESTAMP ELSE project_sandboxes.last_stopped_at END,
                updated_at = CURRENT_TIMESTAMP
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(project_id)
        .bind(user_id)
        .bind(container_name)
        .bind(status)
        .bind(is_running)
        .fetch_one(self.pool)
        .await?;

        Ok(sandbox)
    }

    pub async fn get_project_sandbox(
        &self,
        project_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<ProjectSandbox>> {
        let sandbox = sqlx::query_as::<_, ProjectSandbox>(
            "SELECT * FROM project_sandboxes WHERE project_id = $1 AND user_id = $2",
        )
        .bind(project_id)
        .bind(user_id)
        .fetch_optional(self.pool)
        .await?;
        Ok(sandbox)
    }

    // --- User Session Operations ---

    pub async fn create_user_session(
        &self,
        user_id: Uuid,
        token_hash: &str,
        expires_at: DateTime<Utc>,
        user_agent: Option<&str>,
        ip_address: Option<&str>,
    ) -> Result<UserSession> {
        let id = Uuid::now_v7();
        let session = sqlx::query_as::<_, UserSession>(
            r#"
            INSERT INTO user_sessions (id, user_id, token_hash, expires_at, user_agent, ip_address)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(token_hash)
        .bind(expires_at)
        .bind(user_agent)
        .bind(ip_address)
        .fetch_one(self.pool)
        .await?;

        Ok(session)
    }

    pub async fn get_user_by_session_token_hash(&self, token_hash: &str) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT u.* FROM users u
            JOIN user_sessions s ON u.id = s.user_id
            WHERE s.token_hash = $1 AND s.expires_at > CURRENT_TIMESTAMP AND u.is_active = true
            "#,
        )
        .bind(token_hash)
        .fetch_optional(self.pool)
        .await?;

        Ok(user)
    }

    pub async fn delete_user_session(&self, token_hash: &str) -> Result<()> {
        sqlx::query("DELETE FROM user_sessions WHERE token_hash = $1")
            .bind(token_hash)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    // --- FIDO2 / WebAuthn Credentials ---

    pub async fn save_fido2_credential(
        &self,
        user_id: Uuid,
        credential_id: &str,
        public_key: &[u8],
        counter: i64,
        device_name: &str,
        aaguid: Option<&[u8]>,
    ) -> Result<Fido2Credential> {
        let id = Uuid::now_v7();
        let cred = sqlx::query_as::<_, Fido2Credential>(
            r#"
            INSERT INTO fido2_credentials (id, user_id, credential_id, public_key, counter, device_name, aaguid)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(credential_id)
        .bind(public_key)
        .bind(counter)
        .bind(device_name)
        .bind(aaguid)
        .fetch_one(self.pool)
        .await?;

        Ok(cred)
    }

    pub async fn get_fido2_credentials_by_user(&self, user_id: Uuid) -> Result<Vec<Fido2Credential>> {
        let creds = sqlx::query_as::<_, Fido2Credential>(
            "SELECT * FROM fido2_credentials WHERE user_id = $1 ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(self.pool)
        .await?;
        Ok(creds)
    }

    pub async fn get_fido2_credential_by_id(&self, credential_id: &str) -> Result<Option<Fido2Credential>> {
        let cred = sqlx::query_as::<_, Fido2Credential>(
            "SELECT * FROM fido2_credentials WHERE credential_id = $1",
        )
        .bind(credential_id)
        .fetch_optional(self.pool)
        .await?;
        Ok(cred)
    }

    pub async fn update_fido2_counter(&self, credential_id: &str, counter: i64) -> Result<()> {
        sqlx::query(
            "UPDATE fido2_credentials SET counter = $1, last_used_at = CURRENT_TIMESTAMP WHERE credential_id = $2",
        )
        .bind(counter)
        .bind(credential_id)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    // --- System Settings & Invitations ---

    pub async fn get_system_settings(&self) -> Result<SystemSettings> {
        let settings = sqlx::query_as::<_, SystemSettings>(
            "SELECT * FROM system_settings WHERE id = 1",
        )
        .fetch_one(self.pool)
        .await?;
        Ok(settings)
    }

    pub async fn update_system_settings(&self, dto: UpdateSystemSettingsDto) -> Result<SystemSettings> {
        let settings = sqlx::query_as::<_, SystemSettings>(
            r#"
            UPDATE system_settings
            SET registration_mode = COALESCE($1, registration_mode),
                smtp_host = COALESCE($2, smtp_host),
                smtp_port = COALESCE($3, smtp_port),
                smtp_username = COALESCE($4, smtp_username),
                smtp_password = COALESCE($5, smtp_password),
                smtp_from_email = COALESCE($6, smtp_from_email),
                smtp_from_name = COALESCE($7, smtp_from_name),
                smtp_use_tls = COALESCE($8, smtp_use_tls),
                smtp_enabled = COALESCE($9, smtp_enabled),
                updated_at = CURRENT_TIMESTAMP
            WHERE id = 1
            RETURNING *
            "#,
        )
        .bind(dto.registration_mode)
        .bind(dto.smtp_host)
        .bind(dto.smtp_port)
        .bind(dto.smtp_username)
        .bind(dto.smtp_password)
        .bind(dto.smtp_from_email)
        .bind(dto.smtp_from_name)
        .bind(dto.smtp_use_tls)
        .bind(dto.smtp_enabled)
        .fetch_one(self.pool)
        .await?;

        Ok(settings)
    }

    pub async fn create_invitation(&self, token: &str, dto: CreateInvitationDto) -> Result<Invitation> {
        let id = Uuid::now_v7();
        let role = dto.role.unwrap_or_else(|| "member".to_string());

        let invite = sqlx::query_as::<_, Invitation>(
            r#"
            INSERT INTO invitations (id, token, email, org_id, team_id, role, inviter_id, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(token)
        .bind(&dto.email)
        .bind(dto.org_id)
        .bind(dto.team_id)
        .bind(role)
        .bind(dto.inviter_id)
        .bind(dto.expires_at)
        .fetch_one(self.pool)
        .await?;

        Ok(invite)
    }

    pub async fn get_invitation_by_token(&self, token: &str) -> Result<Option<Invitation>> {
        let invite = sqlx::query_as::<_, Invitation>(
            "SELECT * FROM invitations WHERE token = $1 AND used_at IS NULL AND expires_at > CURRENT_TIMESTAMP",
        )
        .bind(token)
        .fetch_optional(self.pool)
        .await?;
        Ok(invite)
    }

    pub async fn mark_invitation_used(&self, token: &str) -> Result<()> {
        sqlx::query("UPDATE invitations SET used_at = CURRENT_TIMESTAMP WHERE token = $1")
            .bind(token)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    // --- SSO / OAuth2 Platform Operations ---

    pub async fn create_oauth_client(&self, dto: CreateOAuthClientDto) -> Result<OAuthClient> {
        let client = sqlx::query_as::<_, OAuthClient>(
            r#"
            INSERT INTO oauth_clients (client_id, client_secret_hash, name, redirect_uris, is_confidential)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(&dto.client_id)
        .bind(&dto.client_secret_hash)
        .bind(&dto.name)
        .bind(&dto.redirect_uris)
        .bind(dto.is_confidential)
        .fetch_one(self.pool)
        .await?;

        Ok(client)
    }

    pub async fn get_oauth_client_by_id(&self, client_id: &str) -> Result<Option<OAuthClient>> {
        let client = sqlx::query_as::<_, OAuthClient>(
            "SELECT * FROM oauth_clients WHERE client_id = $1",
        )
        .bind(client_id)
        .fetch_optional(self.pool)
        .await?;
        Ok(client)
    }

    pub async fn list_oauth_clients(&self) -> Result<Vec<OAuthClient>> {
        let clients = sqlx::query_as::<_, OAuthClient>(
            "SELECT * FROM oauth_clients ORDER BY created_at DESC",
        )
        .fetch_all(self.pool)
        .await?;
        Ok(clients)
    }

    pub async fn create_oauth_auth_code(
        &self,
        code: &str,
        client_id: &str,
        user_id: Uuid,
        redirect_uri: &str,
        scope: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<OAuthAuthCode> {
        let auth_code = sqlx::query_as::<_, OAuthAuthCode>(
            r#"
            INSERT INTO oauth_auth_codes (code, client_id, user_id, redirect_uri, scope, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(code)
        .bind(client_id)
        .bind(user_id)
        .bind(redirect_uri)
        .bind(scope)
        .bind(expires_at)
        .fetch_one(self.pool)
        .await?;

        Ok(auth_code)
    }

    pub async fn consume_oauth_auth_code(&self, code: &str) -> Result<Option<OAuthAuthCode>> {
        let auth_code = sqlx::query_as::<_, OAuthAuthCode>(
            r#"
            DELETE FROM oauth_auth_codes
            WHERE code = $1 AND expires_at > CURRENT_TIMESTAMP
            RETURNING *
            "#,
        )
        .bind(code)
        .fetch_optional(self.pool)
        .await?;

        Ok(auth_code)
    }
}
