use crate::error::Result;
use crate::models::AuditLog;
use crate::models::CreateAuditLogDto;
use crate::models::CreateDocumentDto;
use crate::models::CreateInvitationDto;
use crate::models::CreateKnowledgeEdgeDto;
use crate::models::CreateKnowledgeNodeDto;
use crate::models::CreateOAuthClientDto;
use crate::models::CreateOrganizationDto;
use crate::models::CreateProjectDto;
use crate::models::CreateTeamDto;
use crate::models::CreateTemplateDto;
use crate::models::CreateUserDto;
use crate::models::CreateWorkspaceDto;
use crate::models::Document;
use crate::models::DocumentSearchResult;
use crate::models::EffectiveHubLinks;
use crate::models::Fido2Credential;
use crate::models::GpgPublicKey;
use crate::models::Invitation;
use crate::models::KnowledgeEdge;
use crate::models::KnowledgeNode;
use crate::models::MemberRole;
use crate::models::OAuthAuthCode;
use crate::models::OAuthClient;
use crate::models::OrgMemberWithUser;
use crate::models::Organization;
use crate::models::PersonalAccessToken;
use crate::models::Project;
use crate::models::ProjectMember;
use crate::models::ProjectMemberWithUser;
use crate::models::ProjectSandbox;
use crate::models::PublishTemplateVersionDto;
use crate::models::SshPublicKey;
use crate::models::SystemSettings;
use crate::models::Team;
use crate::models::TeamMemberWithUser;
use crate::models::TeamTreeNode;
use crate::models::TeamWithOrg;
use crate::models::Template;
use crate::models::TemplateShare;
use crate::models::TemplateVersion;
use crate::models::TemplateWithLatestVersion;
use crate::models::UpdateOrganizationDto;
use crate::models::UpdateSystemSettingsDto;
use crate::models::UpdateTeamDto;
use crate::models::UpdateUserProfileDto;
use crate::models::UpsertGitCredentialDto;
use crate::models::User;
use crate::models::User2faChallenge;
use crate::models::UserGitCredential;
use crate::models::UserOrgMembership;
use crate::models::UserRole;
use crate::models::UserTeamMembership;
use crate::models::UserSession;
use crate::models::Workspace;
use crate::models::WorkspaceMember;
use chrono::DateTime;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

/// Data access layer and repository operations on PostgreSQL.
pub struct Repository<'a> {
    pool: &'a PgPool,
}

#[allow(missing_docs)]
impl<'a> Repository<'a> {
    /// Creates a new repository bound to a PostgreSQL connection pool.
    #[must_use]
    pub const fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    // --- User Operations ---

    /// Create user.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn create_user(
        &self,
        dto: CreateUserDto,
    ) -> Result<User> {
        let id = Uuid::now_v7();
        let role = dto.role.unwrap_or_default();
        let is_platform_admin = dto
            .is_platform_admin
            .unwrap_or(matches!(role, UserRole::Admin));
        let quota = dto.storage_quota_bytes.unwrap_or(100 * 1024 * 1024);

        let user = sqlx::query_as::<_, User>(
            r"
            INSERT INTO users (id, username, email, password_hash, display_name, role, is_platform_admin, storage_quota_bytes)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            ",
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

    /// Get user by ID.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn get_user_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool)
            .await?;
        Ok(user)
    }

    /// Get user by username.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn get_user_by_username(
        &self,
        username: &str,
    ) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
            .bind(username)
            .fetch_optional(self.pool)
            .await?;
        Ok(user)
    }

    /// Update user profile.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn update_user_profile(
        &self,
        id: Uuid,
        dto: UpdateUserProfileDto,
    ) -> Result<User> {
        let user = sqlx::query_as::<_, User>(
            r"
            UPDATE users
            SET display_name = COALESCE($2, display_name),
                email = COALESCE($3, email),
                avatar_url = COALESCE($4, avatar_url),
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $1
            RETURNING *
            ",
        )
        .bind(id)
        .bind(dto.display_name)
        .bind(dto.email)
        .bind(dto.avatar_url)
        .fetch_one(self.pool)
        .await?;

        Ok(user)
    }

    /// Update user password.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn update_user_password(
        &self,
        id: Uuid,
        password_hash: &str,
    ) -> Result<()> {
        sqlx::query(
            r"
            UPDATE users
            SET password_hash = $2,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $1
            ",
        )
        .bind(id)
        .bind(password_hash)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// Toggle user active status and purge active sessions if locked.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn set_user_active(
        &self,
        user_id: Uuid,
        is_active: bool,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            r"
            UPDATE users
            SET is_active = $2,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $1
            ",
        )
        .bind(user_id)
        .bind(is_active)
        .execute(&mut *tx)
        .await?;

        if !is_active {
            sqlx::query("DELETE FROM user_sessions WHERE user_id = $1")
                .bind(user_id)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Update user total storage space limit in bytes.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn update_user_storage_quota(
        &self,
        user_id: Uuid,
        quota_bytes: i64,
    ) -> Result<()> {
        sqlx::query(
            r"
            UPDATE users
            SET storage_quota_bytes = $2,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $1
            ",
        )
        .bind(user_id)
        .bind(quota_bytes)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// Permanently delete a user account and cascade delete associated records.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn delete_user(
        &self,
        user_id: Uuid,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // 1. Delete authentication & session records
        sqlx::query("DELETE FROM user_sessions WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM personal_access_tokens WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM ssh_public_keys WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM gpg_public_keys WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM fido2_credentials WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        // 2. Delete memberships
        sqlx::query("DELETE FROM org_members WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM team_members WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM workspace_members WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM project_members WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        // 3. Delete owned projects and workspaces
        sqlx::query("DELETE FROM projects WHERE owner_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM workspaces WHERE owner_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        // 4. Delete user record
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    /// List all organization memberships for a user.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn list_user_org_memberships(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<UserOrgMembership>> {
        let rows = sqlx::query_as::<_, UserOrgMembership>(
            r"
            SELECT om.org_id, o.name AS org_name, o.slug AS org_slug, om.role
            FROM org_members om
            JOIN organizations o ON om.org_id = o.id
            WHERE om.user_id = $1
            ORDER BY o.name ASC
            ",
        )
        .bind(user_id)
        .fetch_all(self.pool)
        .await?;

        Ok(rows)
    }

    /// List all team memberships for a user.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn list_user_team_memberships(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<UserTeamMembership>> {
        let rows = sqlx::query_as::<_, UserTeamMembership>(
            r"
            SELECT tm.team_id, t.name AS team_name, t.slug AS team_slug, t.org_id, o.name AS org_name, tm.role
            FROM team_members tm
            JOIN teams t ON tm.team_id = t.id
            JOIN organizations o ON t.org_id = o.id
            WHERE tm.user_id = $1
            ORDER BY t.name ASC
            ",
        )
        .bind(user_id)
        .fetch_all(self.pool)
        .await?;

        Ok(rows)
    }

    /// List all teams with their parent organization display name.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn list_all_teams_with_org(&self) -> Result<Vec<TeamWithOrg>> {
        let rows = sqlx::query_as::<_, TeamWithOrg>(
            r"
            SELECT t.id, t.org_id, t.name, t.slug, t.description, o.name AS org_name
            FROM teams t
            JOIN organizations o ON t.org_id = o.id
            ORDER BY o.name ASC, t.name ASC
            ",
        )
        .fetch_all(self.pool)
        .await?;

        Ok(rows)
    }

    /// Retrieve all on-disk storage directory paths for a user's active projects and workspaces.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn get_user_storage_paths(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<String>> {
        let mut paths = Vec::new();
        let proj_rows: Vec<(String,)> = sqlx::query_as(
            "SELECT storage_path FROM projects WHERE owner_id = $1 AND status != 'deleted'",
        )
        .bind(user_id)
        .fetch_all(self.pool)
        .await?;
        paths.extend(proj_rows.into_iter().map(|(p,)| p));

        let ws_rows: Vec<(Option<String>,)> = sqlx::query_as(
            "SELECT storage_path FROM workspaces WHERE owner_id = $1 AND storage_path IS NOT NULL",
        )
        .bind(user_id)
        .fetch_all(self.pool)
        .await?;
        for (p,) in ws_rows {
            if let Some(p) = p {
                paths.push(p);
            }
        }

        Ok(paths)
    }

    // --- Workspace Operations ---

    /// Create workspace.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn create_workspace(
        &self,
        dto: CreateWorkspaceDto,
    ) -> Result<Workspace> {
        let id = dto.id.unwrap_or_else(Uuid::now_v7);
        let visibility = dto.visibility.unwrap_or_default();
        let container_name = format!("apich-ws-{}", dto.slug);
        let settings = dto.settings.unwrap_or_else(|| serde_json::json!({}));

        let mut tx = self.pool.begin().await?;

        let ws = sqlx::query_as::<_, Workspace>(
            r"
            INSERT INTO workspaces (id, owner_id, slug, name, description, visibility, container_name, settings)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            ",
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
            r"
            INSERT INTO workspace_members (workspace_id, user_id, role)
            VALUES ($1, $2, $3)
            ",
        )
        .bind(ws.id)
        .bind(dto.owner_id)
        .bind(MemberRole::Owner)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(ws)
    }

    /// Get workspace by ID.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn get_workspace_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<Workspace>> {
        let ws = sqlx::query_as::<_, Workspace>("SELECT * FROM workspaces WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool)
            .await?;
        Ok(ws)
    }

    /// Add workspace member.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn add_workspace_member(
        &self,
        workspace_id: Uuid,
        user_id: Uuid,
        role: MemberRole,
    ) -> Result<WorkspaceMember> {
        let member = sqlx::query_as::<_, WorkspaceMember>(
            r"
            INSERT INTO workspace_members (workspace_id, user_id, role)
            VALUES ($1, $2, $3)
            ON CONFLICT (workspace_id, user_id) DO UPDATE SET role = EXCLUDED.role
            RETURNING *
            ",
        )
        .bind(workspace_id)
        .bind(user_id)
        .bind(role)
        .fetch_one(self.pool)
        .await?;

        Ok(member)
    }

    // --- Document Operations ---

    /// Create document.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn create_document(
        &self,
        dto: CreateDocumentDto,
    ) -> Result<Document> {
        let id = dto.id.unwrap_or_else(Uuid::now_v7);
        let content = dto.content.unwrap_or_default();
        let doc_type = dto.doc_type.unwrap_or_default();
        let metadata = dto.metadata.unwrap_or_else(|| serde_json::json!({}));

        let doc = sqlx::query_as::<_, Document>(
            r"
            INSERT INTO documents (id, workspace_id, rel_path, title, content, doc_type, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            ",
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
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn upsert_document(
        &self,
        dto: CreateDocumentDto,
    ) -> Result<Document> {
        let id = dto.id.unwrap_or_else(Uuid::now_v7);
        let content = dto.content.unwrap_or_default();
        let doc_type = dto.doc_type.unwrap_or_default();
        let metadata = dto.metadata.unwrap_or_else(|| serde_json::json!({}));

        let doc = sqlx::query_as::<_, Document>(
            r"
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
            ",
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

    /// Get document by path.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
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
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn search_documents_fulltext(
        &self,
        workspace_id: Uuid,
        query: &str,
        limit: i64,
    ) -> Result<Vec<DocumentSearchResult>> {
        let results = sqlx::query_as::<_, DocumentSearchResult>(
            r"
            SELECT id, workspace_id, rel_path, title, content, doc_type, version, metadata, created_at, updated_at,
                   ts_rank(search_tokens, websearch_to_tsquery('english', $2)) AS rank
            FROM documents
            WHERE workspace_id = $1 AND search_tokens @@ websearch_to_tsquery('english', $2)
            ORDER BY rank DESC, updated_at DESC
            LIMIT $3;
            ",
        )
        .bind(workspace_id)
        .bind(query)
        .bind(limit)
        .fetch_all(self.pool)
        .await?;

        Ok(results)
    }

    /// Query documents matching JSONB metadata containment (`@>`), accelerated by GIN `jsonb_path_ops`
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn find_documents_by_metadata(
        &self,
        workspace_id: Uuid,
        filter: &serde_json::Value,
    ) -> Result<Vec<Document>> {
        let docs = sqlx::query_as::<_, Document>(
            r"
            SELECT * FROM documents
            WHERE workspace_id = $1 AND metadata @> $2
            ORDER BY updated_at DESC;
            ",
        )
        .bind(workspace_id)
        .bind(filter)
        .fetch_all(self.pool)
        .await?;

        Ok(docs)
    }

    /// Extract array elements from JSONB metadata using standard SQL/JSON `json_table` (PostgreSQL 17/18)
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn extract_document_tags(
        &self,
        doc_id: Uuid,
    ) -> Result<Vec<String>> {
        let tags: Vec<String> = sqlx::query_scalar(
            r"
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
            ",
        )
        .bind(doc_id)
        .fetch_all(self.pool)
        .await?;

        Ok(tags)
    }

    /// Extract creation timestamp from `UUIDv7` natively via PostgreSQL 18 function `uuid_extract_timestamp`
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn extract_uuidv7_timestamp(
        &self,
        id: Uuid,
    ) -> Result<DateTime<Utc>> {
        let ts: DateTime<Utc> = sqlx::query_scalar("SELECT uuid_extract_timestamp($1);")
            .bind(id)
            .fetch_one(self.pool)
            .await?;

        Ok(ts)
    }

    // --- Knowledge Graph Operations (Plan.md Section 4) ---

    /// Create knowledge node.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn create_knowledge_node(
        &self,
        dto: CreateKnowledgeNodeDto,
    ) -> Result<KnowledgeNode> {
        let id = dto.id.unwrap_or_else(Uuid::now_v7);
        let metadata = dto.metadata.unwrap_or_else(|| serde_json::json!({}));

        let node = sqlx::query_as::<_, KnowledgeNode>(
            r"
            INSERT INTO knowledge_nodes (id, workspace_id, node_type, title, content_hash, metadata)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            ",
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

    /// Create knowledge edge.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn create_knowledge_edge(
        &self,
        dto: CreateKnowledgeEdgeDto,
    ) -> Result<KnowledgeEdge> {
        let id = dto.id.unwrap_or_else(Uuid::now_v7);
        let weight = dto.weight.unwrap_or(1.0);

        let edge = sqlx::query_as::<_, KnowledgeEdge>(
            r"
            INSERT INTO knowledge_edges (id, workspace_id, source_id, target_id, relation_type, weight)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            ",
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

    /// Record audit log.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn record_audit_log(
        &self,
        dto: CreateAuditLogDto,
    ) -> Result<AuditLog> {
        let id = Uuid::now_v7();

        let log = sqlx::query_as::<_, AuditLog>(
            r"
            INSERT INTO audit_logs (id, user_id, workspace_id, action, details, ip_address)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            ",
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

    /// Get user by email.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn get_user_by_email(
        &self,
        email: &str,
    ) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
            .bind(email)
            .fetch_optional(self.pool)
            .await?;
        Ok(user)
    }

    /// List users.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn list_users(&self) -> Result<Vec<User>> {
        let users = sqlx::query_as::<_, User>("SELECT * FROM users ORDER BY created_at DESC")
            .fetch_all(self.pool)
            .await?;
        Ok(users)
    }

    /// Search active users by username, display name, or email prefix/containment.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn search_users(&self, query: &str, limit: i64) -> Result<Vec<User>> {
        let limit = limit.clamp(1, 50);
        let q = query.trim();
        if q.is_empty() {
            let users = sqlx::query_as::<_, User>(
                "SELECT * FROM users WHERE is_active = true ORDER BY username ASC LIMIT $1",
            )
            .bind(limit)
            .fetch_all(self.pool)
            .await?;
            return Ok(users);
        }

        let pattern = format!("%{q}%");
        let prefix = format!("{q}%");
        let users = sqlx::query_as::<_, User>(
            r#"
            SELECT * FROM users
            WHERE is_active = true
              AND (username ILIKE $1 OR display_name ILIKE $1 OR email ILIKE $1)
            ORDER BY
              CASE
                WHEN username ILIKE $2 THEN 0
                WHEN display_name ILIKE $2 THEN 1
                ELSE 2
              END,
              username ASC
            LIMIT $3
            "#,
        )
        .bind(&pattern)
        .bind(&prefix)
        .bind(limit)
        .fetch_all(self.pool)
        .await?;
        Ok(users)
    }

    // --- Organization Operations ---

    /// Create organizationanization.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn create_organization(
        &self,
        owner_id: Uuid,
        dto: CreateOrganizationDto,
    ) -> Result<Organization> {
        let id = Uuid::now_v7();
        let mut tx = self.pool.begin().await?;
        let allow_team_override = dto.allow_team_override.unwrap_or(true);

        let org = sqlx::query_as::<_, Organization>(
            r"
            INSERT INTO organizations (id, slug, name, description, chat_url, meeting_url, drive_url, ai_agent_url, allow_team_override)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            ",
        )
        .bind(id)
        .bind(&dto.slug)
        .bind(&dto.name)
        .bind(&dto.description)
        .bind(&dto.chat_url)
        .bind(&dto.meeting_url)
        .bind(&dto.drive_url)
        .bind(&dto.ai_agent_url)
        .bind(allow_team_override)
        .fetch_one(&mut *tx)
        .await?;

        // Add creator as organization owner
        sqlx::query(
            r"
            INSERT INTO org_members (org_id, user_id, role)
            VALUES ($1, $2, 'owner')
            ",
        )
        .bind(org.id)
        .bind(owner_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(org)
    }

    /// Get organizationanization by ID.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn get_organization_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<Organization>> {
        let org = sqlx::query_as::<_, Organization>("SELECT * FROM organizations WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool)
            .await?;
        Ok(org)
    }

    /// Get organizationanization by slug.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn get_organization_by_slug(
        &self,
        slug: &str,
    ) -> Result<Option<Organization>> {
        let org = sqlx::query_as::<_, Organization>("SELECT * FROM organizations WHERE slug = $1")
            .bind(slug)
            .fetch_optional(self.pool)
            .await?;
        Ok(org)
    }

    /// List organizationanizations for user.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn list_organizations_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<Organization>> {
        let orgs = sqlx::query_as::<_, Organization>(
            r"
            SELECT o.* FROM organizations o
            JOIN org_members om ON o.id = om.org_id
            WHERE om.user_id = $1
            ORDER BY o.name ASC
            ",
        )
        .bind(user_id)
        .fetch_all(self.pool)
        .await?;
        Ok(orgs)
    }

    /// List organizationanizations.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn list_organizations(
        &self,
        limit: i64,
    ) -> Result<Vec<Organization>> {
        let orgs = sqlx::query_as::<_, Organization>(
            "SELECT * FROM organizations ORDER BY name ASC LIMIT $1",
        )
        .bind(limit)
        .fetch_all(self.pool)
        .await?;
        Ok(orgs)
    }

    /// Add organization member.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn add_org_member(
        &self,
        org_id: Uuid,
        user_id: Uuid,
        role: &str,
    ) -> Result<()> {
        sqlx::query(
            r"
            INSERT INTO org_members (org_id, user_id, role)
            VALUES ($1, $2, $3)
            ON CONFLICT (org_id, user_id) DO UPDATE SET role = EXCLUDED.role
            ",
        )
        .bind(org_id)
        .bind(user_id)
        .bind(role)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Remove organization member.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn remove_org_member(
        &self,
        org_id: Uuid,
        user_id: Uuid,
    ) -> Result<()> {
        sqlx::query("DELETE FROM org_members WHERE org_id = $1 AND user_id = $2")
            .bind(org_id)
            .bind(user_id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// List organization members.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn list_org_members(
        &self,
        org_id: Uuid,
    ) -> Result<Vec<OrgMemberWithUser>> {
        let rows = sqlx::query_as::<_, (Uuid, Uuid, String, String, String, String, Option<String>, DateTime<Utc>)>(
            r"
            SELECT om.org_id, om.user_id, om.role, u.username, u.email, u.display_name, u.avatar_url, om.joined_at
            FROM org_members om
            JOIN users u ON om.user_id = u.id
            WHERE om.org_id = $1
            ORDER BY om.joined_at ASC
            ",
        )
        .bind(org_id)
        .fetch_all(self.pool)
        .await?;

        let members = rows
            .into_iter()
            .map(
                |(org_id, user_id, role, username, email, display_name, avatar_url, joined_at)| {
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
                },
            )
            .collect();

        Ok(members)
    }

    /// Update organizationanization.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn update_organization(
        &self,
        id: Uuid,
        dto: UpdateOrganizationDto,
    ) -> Result<Organization> {
        let org = sqlx::query_as::<_, Organization>(
            r"
            UPDATE organizations
            SET name = COALESCE($2, name),
                slug = COALESCE($3, slug),
                description = COALESCE($4, description),
                chat_url = CASE WHEN $5::text IS NOT NULL THEN NULLIF($5, '') ELSE chat_url END,
                meeting_url = CASE WHEN $6::text IS NOT NULL THEN NULLIF($6, '') ELSE meeting_url END,
                drive_url = CASE WHEN $7::text IS NOT NULL THEN NULLIF($7, '') ELSE drive_url END,
                ai_agent_url = CASE WHEN $8::text IS NOT NULL THEN NULLIF($8, '') ELSE ai_agent_url END,
                allow_team_override = COALESCE($9, allow_team_override),
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $1
            RETURNING *
            ",
        )
        .bind(id)
        .bind(dto.name)
        .bind(dto.slug)
        .bind(dto.description)
        .bind(dto.chat_url)
        .bind(dto.meeting_url)
        .bind(dto.drive_url)
        .bind(dto.ai_agent_url)
        .bind(dto.allow_team_override)
        .fetch_one(self.pool)
        .await?;

        Ok(org)
    }

    /// Delete organizationanization.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn delete_organization(
        &self,
        id: Uuid,
    ) -> Result<()> {
        sqlx::query("DELETE FROM organizations WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// Resolve effective external hub links (Chat, Video, Drive, AI) following recursive override rules.
    ///
    /// Rules:
    /// - If Org has `allow_team_override == false`, Org links are strictly enforced globally.
    /// - If Org has `allow_team_override == true` and `team_id` is provided, traverses the team hierarchy
    ///   upwards to find team-level customizations, falling back to Org links for any unconfigured items.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn resolve_hub_links(
        &self,
        org_id: Uuid,
        team_id: Option<Uuid>,
    ) -> Result<EffectiveHubLinks> {
        let Some(org) = self.get_organization_by_id(org_id).await? else {
            return Ok(EffectiveHubLinks::default());
        };

        if !org.allow_team_override || team_id.is_none() {
            return Ok(EffectiveHubLinks {
                chat_url: org.chat_url,
                meeting_url: org.meeting_url,
                drive_url: org.drive_url,
                ai_agent_url: org.ai_agent_url,
                is_team_override: false,
                allow_team_override: org.allow_team_override,
            });
        }

        let mut curr_team_id = team_id;
        let mut chat_url = None;
        let mut meeting_url = None;
        let mut drive_url = None;
        let mut ai_agent_url = None;
        let mut is_team_override = false;

        // Traverse team hierarchy upwards from child to parents
        while let Some(tid) = curr_team_id {
            if let Some(t) = self.get_team_by_id(tid).await? {
                if chat_url.is_none() && t.chat_url.is_some() {
                    chat_url = t.chat_url;
                    is_team_override = true;
                }
                if meeting_url.is_none() && t.meeting_url.is_some() {
                    meeting_url = t.meeting_url;
                    is_team_override = true;
                }
                if drive_url.is_none() && t.drive_url.is_some() {
                    drive_url = t.drive_url;
                    is_team_override = true;
                }
                if ai_agent_url.is_none() && t.ai_agent_url.is_some() {
                    ai_agent_url = t.ai_agent_url;
                    is_team_override = true;
                }
                curr_team_id = t.parent_team_id;
            } else {
                break;
            }
        }

        Ok(EffectiveHubLinks {
            chat_url: chat_url.or(org.chat_url),
            meeting_url: meeting_url.or(org.meeting_url),
            drive_url: drive_url.or(org.drive_url),
            ai_agent_url: ai_agent_url.or(org.ai_agent_url),
            is_team_override,
            allow_team_override: true,
        })
    }

    // --- Team Operations (Recursive tree) ---

    /// Create team.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn create_team(
        &self,
        creator_id: Uuid,
        dto: CreateTeamDto,
    ) -> Result<Team> {
        let id = Uuid::now_v7();
        let mut tx = self.pool.begin().await?;

        let team = sqlx::query_as::<_, Team>(
            r"
            INSERT INTO teams (id, org_id, parent_team_id, name, slug, description, chat_url, meeting_url, drive_url, ai_agent_url)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            ",
        )
        .bind(id)
        .bind(dto.org_id)
        .bind(dto.parent_team_id)
        .bind(&dto.name)
        .bind(&dto.slug)
        .bind(&dto.description)
        .bind(&dto.chat_url)
        .bind(&dto.meeting_url)
        .bind(&dto.drive_url)
        .bind(&dto.ai_agent_url)
        .fetch_one(&mut *tx)
        .await?;

        // Creator becomes team admin
        sqlx::query(
            r"
            INSERT INTO team_members (team_id, user_id, role)
            VALUES ($1, $2, 'admin')
            ON CONFLICT (team_id, user_id) DO UPDATE SET role = 'admin'
            ",
        )
        .bind(team.id)
        .bind(creator_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(team)
    }

    /// Get team by ID.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn get_team_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<Team>> {
        let team = sqlx::query_as::<_, Team>("SELECT * FROM teams WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool)
            .await?;
        Ok(team)
    }

    /// List teams by organization.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn list_teams_by_org(
        &self,
        org_id: Uuid,
    ) -> Result<Vec<Team>> {
        let teams = sqlx::query_as::<_, Team>(
            "SELECT * FROM teams WHERE org_id = $1 ORDER BY parent_team_id NULLS FIRST, name ASC",
        )
        .bind(org_id)
        .fetch_all(self.pool)
        .await?;
        Ok(teams)
    }

    /// Build hierarchical team tree for an organization
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn get_team_tree_for_org(
        &self,
        org_id: Uuid,
    ) -> Result<Vec<TeamTreeNode>> {
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

        let all_teams = self.list_teams_by_org(org_id).await?;
        Ok(assemble_subtrees(None, &all_teams))
    }

    /// Add team member.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn add_team_member(
        &self,
        team_id: Uuid,
        user_id: Uuid,
        role: &str,
    ) -> Result<()> {
        sqlx::query(
            r"
            INSERT INTO team_members (team_id, user_id, role)
            VALUES ($1, $2, $3)
            ON CONFLICT (team_id, user_id) DO UPDATE SET role = EXCLUDED.role
            ",
        )
        .bind(team_id)
        .bind(user_id)
        .bind(role)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Remove team member.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn remove_team_member(
        &self,
        team_id: Uuid,
        user_id: Uuid,
    ) -> Result<()> {
        sqlx::query("DELETE FROM team_members WHERE team_id = $1 AND user_id = $2")
            .bind(team_id)
            .bind(user_id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// List team members.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn list_team_members(
        &self,
        team_id: Uuid,
    ) -> Result<Vec<TeamMemberWithUser>> {
        let rows = sqlx::query_as::<_, (Uuid, Uuid, String, String, String, String, Option<String>, DateTime<Utc>)>(
            r"
            SELECT tm.team_id, tm.user_id, tm.role, u.username, u.email, u.display_name, u.avatar_url, tm.joined_at
            FROM team_members tm
            JOIN users u ON tm.user_id = u.id
            WHERE tm.team_id = $1
            ORDER BY tm.joined_at ASC
            ",
        )
        .bind(team_id)
        .fetch_all(self.pool)
        .await?;

        let members = rows
            .into_iter()
            .map(
                |(team_id, user_id, role, username, email, display_name, avatar_url, joined_at)| {
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
                },
            )
            .collect();

        Ok(members)
    }

    /// Update team.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn update_team(
        &self,
        id: Uuid,
        dto: UpdateTeamDto,
    ) -> Result<Team> {
        let (update_parent, parent_id) = match dto.parent_team_id {
            | Some(p) => (true, p),
            | None => (false, None),
        };

        let team = sqlx::query_as::<_, Team>(
            r"
            UPDATE teams
            SET name = COALESCE($2, name),
                slug = COALESCE($3, slug),
                description = COALESCE($4, description),
                parent_team_id = CASE WHEN $5 = true THEN $6 ELSE parent_team_id END,
                chat_url = CASE WHEN $7::text IS NOT NULL THEN NULLIF($7, '') ELSE chat_url END,
                meeting_url = CASE WHEN $8::text IS NOT NULL THEN NULLIF($8, '') ELSE meeting_url END,
                drive_url = CASE WHEN $9::text IS NOT NULL THEN NULLIF($9, '') ELSE drive_url END,
                ai_agent_url = CASE WHEN $10::text IS NOT NULL THEN NULLIF($10, '') ELSE ai_agent_url END,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $1
            RETURNING *
            ",
        )
        .bind(id)
        .bind(dto.name)
        .bind(dto.slug)
        .bind(dto.description)
        .bind(update_parent)
        .bind(parent_id)
        .bind(dto.chat_url)
        .bind(dto.meeting_url)
        .bind(dto.drive_url)
        .bind(dto.ai_agent_url)
        .fetch_one(self.pool)
        .await?;

        Ok(team)
    }

    /// Delete team.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn delete_team(
        &self,
        id: Uuid,
    ) -> Result<()> {
        sqlx::query("DELETE FROM teams WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    // --- Project Operations (Core Unit) ---

    /// Create project.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn create_project(
        &self,
        dto: CreateProjectDto,
    ) -> Result<Project> {
        let id = Uuid::now_v7();
        let settings = dto.settings.unwrap_or_else(|| serde_json::json!({}));

        let proj = sqlx::query_as::<_, Project>(
            r"
            INSERT INTO projects (id, org_id, team_id, owner_id, name, slug, description, storage_path, settings)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            ",
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

    /// Get project by ID.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn get_project_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<Project>> {
        let proj = sqlx::query_as::<_, Project>("SELECT * FROM projects WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool)
            .await?;
        Ok(proj)
    }

    /// Get project by slug.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn get_project_by_slug(
        &self,
        org_id: Uuid,
        slug: &str,
    ) -> Result<Option<Project>> {
        let proj =
            sqlx::query_as::<_, Project>("SELECT * FROM projects WHERE org_id = $1 AND slug = $2")
                .bind(org_id)
                .bind(slug)
                .fetch_optional(self.pool)
                .await?;
        Ok(proj)
    }

    /// List projects for user.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn list_projects_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<Project>> {
        let projs = sqlx::query_as::<_, Project>(
            r"
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
            ",
        )
        .bind(user_id)
        .fetch_all(self.pool)
        .await?;
        Ok(projs)
    }

    /// List all projects admin.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn list_all_projects_admin(&self) -> Result<Vec<Project>> {
        let projs = sqlx::query_as::<_, Project>(
            "SELECT * FROM projects WHERE status != 'deleted' ORDER BY updated_at DESC",
        )
        .fetch_all(self.pool)
        .await?;
        Ok(projs)
    }

    /// Update project status.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn update_project_status(
        &self,
        id: Uuid,
        status: &str,
    ) -> Result<()> {
        sqlx::query("UPDATE projects SET status = $1 WHERE id = $2")
            .bind(status)
            .bind(id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// Set project vcs initialized.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn set_project_vcs_initialized(
        &self,
        id: Uuid,
        initialized: bool,
    ) -> Result<()> {
        sqlx::query("UPDATE projects SET vcs_initialized = $1 WHERE id = $2")
            .bind(initialized)
            .bind(id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// Rename a project (display name and description only -- the slug and storage path stay
    /// put, so existing links and the on-disk workspace directory keep working).
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn update_project_name(
        &self,
        id: Uuid,
        name: &str,
        description: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE projects SET name = $1, description = $2, updated_at = CURRENT_TIMESTAMP \
             WHERE id = $3",
        )
        .bind(name)
        .bind(description)
        .bind(id)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Update project settings.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn update_project_settings(
        &self,
        id: Uuid,
        settings: serde_json::Value,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE projects SET settings = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2",
        )
        .bind(settings)
        .bind(id)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    // --- Project Members & Sharing ---

    /// Add project member.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn add_project_member(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        role: &str,
    ) -> Result<ProjectMember> {
        let member = sqlx::query_as::<_, ProjectMember>(
            r"
            INSERT INTO project_members (project_id, user_id, role)
            VALUES ($1, $2, $3)
            ON CONFLICT (project_id, user_id) DO UPDATE SET role = EXCLUDED.role, updated_at = CURRENT_TIMESTAMP
            RETURNING *
            ",
        )
        .bind(project_id)
        .bind(user_id)
        .bind(role)
        .fetch_one(self.pool)
        .await?;

        Ok(member)
    }

    /// Update project member role.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn update_project_member_role(
        &self,
        project_id: Uuid,
        user_id: Uuid,
        role: &str,
    ) -> Result<ProjectMember> {
        let member = sqlx::query_as::<_, ProjectMember>(
            r"
            UPDATE project_members
            SET role = $1, updated_at = CURRENT_TIMESTAMP
            WHERE project_id = $2 AND user_id = $3
            RETURNING *
            ",
        )
        .bind(role)
        .bind(project_id)
        .bind(user_id)
        .fetch_one(self.pool)
        .await?;

        Ok(member)
    }

    /// Remove project member.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
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

    /// List project members.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn list_project_members(
        &self,
        project_id: Uuid,
    ) -> Result<Vec<ProjectMemberWithUser>> {
        let members = sqlx::query_as::<_, ProjectMemberWithUser>(
            r"
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
            ",
        )
        .bind(project_id)
        .fetch_all(self.pool)
        .await?;

        Ok(members)
    }

    /// Get project member.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
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

    /// Upsert project sandbox.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
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
            r"
            INSERT INTO project_sandboxes (id, project_id, user_id, container_name, status, last_started_at, last_activity_at)
            VALUES ($1, $2, $3, $4, $5, CASE WHEN $6 THEN CURRENT_TIMESTAMP ELSE NULL END, CASE WHEN $6 THEN CURRENT_TIMESTAMP ELSE NULL END)
            ON CONFLICT (project_id, user_id) DO UPDATE
            SET container_name = EXCLUDED.container_name,
                status = EXCLUDED.status,
                last_started_at = CASE WHEN $6 THEN CURRENT_TIMESTAMP ELSE project_sandboxes.last_started_at END,
                last_stopped_at = CASE WHEN NOT $6 THEN CURRENT_TIMESTAMP ELSE project_sandboxes.last_stopped_at END,
                last_activity_at = CASE WHEN $6 THEN CURRENT_TIMESTAMP ELSE project_sandboxes.last_activity_at END,
                updated_at = CURRENT_TIMESTAMP
            RETURNING *
            ",
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

    /// Get project sandbox.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
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

    /// Record real activity in a running sandbox (a terminal command, script run, or agent run),
    /// resetting its idle clock. The idle reaper (`ProjectManagerService::reap_idle_sandboxes`)
    /// stops any `running` sandbox whose activity is older than the configured idle timeout.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn touch_sandbox_activity(
        &self,
        project_id: Uuid,
        user_id: Uuid,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE project_sandboxes SET last_activity_at = CURRENT_TIMESTAMP WHERE project_id = $1 AND user_id = $2",
        )
        .bind(project_id)
        .bind(user_id)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// All `running` sandboxes whose last real activity (or, if none was ever recorded, their
    /// start time) is older than `idle_since` -- candidates for the idle reaper to stop.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn list_idle_running_sandboxes(
        &self,
        idle_since: DateTime<Utc>,
    ) -> Result<Vec<ProjectSandbox>> {
        let rows = sqlx::query_as::<_, ProjectSandbox>(
            "SELECT * FROM project_sandboxes WHERE status = 'running' AND COALESCE(last_activity_at, last_started_at) < $1",
        )
        .bind(idle_since)
        .fetch_all(self.pool)
        .await?;
        Ok(rows)
    }

    /// All `stopped` sandboxes that have sat stopped for longer than `stopped_since` -- candidates
    /// for actual removal (`podman rm`), not just stopping. The idle reaper only ever stops a
    /// running container; nothing previously deleted a stopped one, so every sandbox this app ever
    /// started accumulated on the host forever once idle-stopped -- confirmed live: dozens of
    /// `apich-proj-*` containers going back over a day, none of them removed.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn list_stale_stopped_sandboxes(
        &self,
        stopped_since: DateTime<Utc>,
    ) -> Result<Vec<ProjectSandbox>> {
        let rows = sqlx::query_as::<_, ProjectSandbox>(
            "SELECT * FROM project_sandboxes WHERE status = 'stopped' AND last_stopped_at IS NOT NULL AND last_stopped_at < $1",
        )
        .bind(stopped_since)
        .fetch_all(self.pool)
        .await?;
        Ok(rows)
    }

    /// Every tracked (project, user) -> `container_name` mapping regardless of status -- used to
    /// cross-reference against the real, ground-truth list of containers podman reports, so a
    /// container that exists on the host but was never recorded here (or whose record was already
    /// deleted) can be recognized as orphaned and removed.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn list_all_sandbox_container_names(&self) -> Result<Vec<String>> {
        let rows: Vec<(String,)> = sqlx::query_as("SELECT container_name FROM project_sandboxes")
            .fetch_all(self.pool)
            .await?;
        Ok(rows.into_iter().map(|(n,)| n).collect())
    }

    /// Removes a sandbox's tracking row entirely, once its container has actually been removed
    /// from the host -- leaving a stale row around (even with `status = 'stopped'`) after the real
    /// container is gone serves no purpose and only risks a future stale-container mixup.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn delete_project_sandbox(
        &self,
        project_id: Uuid,
        user_id: Uuid,
    ) -> Result<()> {
        sqlx::query("DELETE FROM project_sandboxes WHERE project_id = $1 AND user_id = $2")
            .bind(project_id)
            .bind(user_id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    // --- User Session Operations ---

    /// Create user session.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
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
            r"
            INSERT INTO user_sessions (id, user_id, token_hash, expires_at, user_agent, ip_address)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            ",
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

    /// Get user by session token hash.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn get_user_by_session_token_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            r"
            SELECT u.* FROM users u
            JOIN user_sessions s ON u.id = s.user_id
            WHERE s.token_hash = $1 AND s.expires_at > CURRENT_TIMESTAMP AND u.is_active = true
            ",
        )
        .bind(token_hash)
        .fetch_optional(self.pool)
        .await?;

        Ok(user)
    }

    /// Delete user session.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn delete_user_session(
        &self,
        token_hash: &str,
    ) -> Result<()> {
        sqlx::query("DELETE FROM user_sessions WHERE token_hash = $1")
            .bind(token_hash)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    // --- FIDO2 / WebAuthn Credentials ---

    /// Save fIDo2 credential.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn save_fido2_credential(
        &self,
        user_id: Uuid,
        credential_id: &str,
        public_key: &[u8],
        counter: i64,
        device_name: &str,
        aaguid: Option<&[u8]>,
        passkey_json: Option<&str>,
    ) -> Result<Fido2Credential> {
        let id = Uuid::now_v7();
        let cred = sqlx::query_as::<_, Fido2Credential>(
            r"
            INSERT INTO fido2_credentials (id, user_id, credential_id, public_key, counter, device_name, aaguid, passkey_json)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            ",
        )
        .bind(id)
        .bind(user_id)
        .bind(credential_id)
        .bind(public_key)
        .bind(counter)
        .bind(device_name)
        .bind(aaguid)
        .bind(passkey_json)
        .fetch_one(self.pool)
        .await?;

        Ok(cred)
    }

    /// Get fIDo2 credentials by user.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn get_fido2_credentials_by_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<Fido2Credential>> {
        let creds = sqlx::query_as::<_, Fido2Credential>(
            "SELECT * FROM fido2_credentials WHERE user_id = $1 ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(self.pool)
        .await?;
        Ok(creds)
    }

    /// Get fIDo2 credential by ID.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn get_fido2_credential_by_id(
        &self,
        credential_id: &str,
    ) -> Result<Option<Fido2Credential>> {
        let cred = sqlx::query_as::<_, Fido2Credential>(
            "SELECT * FROM fido2_credentials WHERE credential_id = $1",
        )
        .bind(credential_id)
        .fetch_optional(self.pool)
        .await?;
        Ok(cred)
    }

    /// Update fIDo2 counter.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn update_fido2_counter(
        &self,
        credential_id: &str,
        counter: i64,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE fido2_credentials SET counter = $1, last_used_at = CURRENT_TIMESTAMP WHERE credential_id = $2",
        )
        .bind(counter)
        .bind(credential_id)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Update FIDO2 passkey state, signature counter, and last used timestamp.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn update_fido2_passkey(
        &self,
        credential_id: &str,
        counter: i64,
        passkey_json: &str,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE fido2_credentials SET counter = $1, passkey_json = $2, last_used_at = CURRENT_TIMESTAMP WHERE credential_id = $3",
        )
        .bind(counter)
        .bind(passkey_json)
        .bind(credential_id)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Delete a FIDO2 passkey credential owned by a user.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn delete_fido2_credential(
        &self,
        user_id: Uuid,
        id: Uuid,
    ) -> Result<()> {
        sqlx::query("DELETE FROM fido2_credentials WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    // --- Personal Access Tokens ---

    /// Create personal access token.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn create_personal_access_token(
        &self,
        user_id: Uuid,
        name: &str,
        token_hash: &str,
        token_prefix: &str,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<PersonalAccessToken> {
        let id = Uuid::now_v7();
        let pat = sqlx::query_as::<_, PersonalAccessToken>(
            r"
            INSERT INTO personal_access_tokens (id, user_id, name, token_hash, token_prefix, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            ",
        )
        .bind(id)
        .bind(user_id)
        .bind(name)
        .bind(token_hash)
        .bind(token_prefix)
        .bind(expires_at)
        .fetch_one(self.pool)
        .await?;
        Ok(pat)
    }

    /// List personal access tokens.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn list_personal_access_tokens(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<PersonalAccessToken>> {
        let tokens = sqlx::query_as::<_, PersonalAccessToken>(
            "SELECT * FROM personal_access_tokens WHERE user_id = $1 ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(self.pool)
        .await?;
        Ok(tokens)
    }

    /// Look up the active user behind a plaintext PAT's hash, for Basic/Bearer auth on the
    /// self-hosted git and apich-vcs remote endpoints. Touches `last_used_at` on a hit.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn get_user_by_active_pat_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            r"
            SELECT u.* FROM users u
            JOIN personal_access_tokens t ON u.id = t.user_id
            WHERE t.token_hash = $1
              AND t.revoked_at IS NULL
              AND (t.expires_at IS NULL OR t.expires_at > CURRENT_TIMESTAMP)
              AND u.is_active = true
            ",
        )
        .bind(token_hash)
        .fetch_optional(self.pool)
        .await?;

        if user.is_some() {
            let _ = sqlx::query(
                "UPDATE personal_access_tokens SET last_used_at = CURRENT_TIMESTAMP WHERE token_hash = $1",
            )
            .bind(token_hash)
            .execute(self.pool)
            .await;
        }
        Ok(user)
    }

    /// Revoke personal access token.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn revoke_personal_access_token(
        &self,
        user_id: Uuid,
        token_id: Uuid,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE personal_access_tokens SET revoked_at = CURRENT_TIMESTAMP WHERE id = $1 AND user_id = $2",
        )
        .bind(token_id)
        .bind(user_id)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    // --- SSH Public Keys (storage/identity only -- no SSH transport server) ---

    /// Add ssh public key.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn add_ssh_public_key(
        &self,
        user_id: Uuid,
        name: &str,
        key_type: &str,
        public_key: &str,
        fingerprint: &str,
    ) -> Result<SshPublicKey> {
        let id = Uuid::now_v7();
        let key = sqlx::query_as::<_, SshPublicKey>(
            r"
            INSERT INTO ssh_public_keys (id, user_id, name, key_type, public_key, fingerprint)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            ",
        )
        .bind(id)
        .bind(user_id)
        .bind(name)
        .bind(key_type)
        .bind(public_key)
        .bind(fingerprint)
        .fetch_one(self.pool)
        .await?;
        Ok(key)
    }

    /// List ssh public keys.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn list_ssh_public_keys(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<SshPublicKey>> {
        let keys = sqlx::query_as::<_, SshPublicKey>(
            "SELECT * FROM ssh_public_keys WHERE user_id = $1 ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(self.pool)
        .await?;
        Ok(keys)
    }

    /// Delete ssh public key.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn delete_ssh_public_key(
        &self,
        user_id: Uuid,
        key_id: Uuid,
    ) -> Result<()> {
        sqlx::query("DELETE FROM ssh_public_keys WHERE id = $1 AND user_id = $2")
            .bind(key_id)
            .bind(user_id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    // --- GPG Public Keys (signature verification for apich-vcs snapshots) ---

    /// Add gpg public key.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn add_gpg_public_key(
        &self,
        user_id: Uuid,
        name: &str,
        public_key: &str,
        fingerprint: &str,
    ) -> Result<GpgPublicKey> {
        let id = Uuid::now_v7();
        let key = sqlx::query_as::<_, GpgPublicKey>(
            r"
            INSERT INTO gpg_public_keys (id, user_id, name, public_key, fingerprint)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            ",
        )
        .bind(id)
        .bind(user_id)
        .bind(name)
        .bind(public_key)
        .bind(fingerprint)
        .fetch_one(self.pool)
        .await?;
        Ok(key)
    }

    /// List gpg public keys.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn list_gpg_public_keys(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<GpgPublicKey>> {
        let keys = sqlx::query_as::<_, GpgPublicKey>(
            "SELECT * FROM gpg_public_keys WHERE user_id = $1 ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(self.pool)
        .await?;
        Ok(keys)
    }

    /// Delete gpg public key.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn delete_gpg_public_key(
        &self,
        user_id: Uuid,
        key_id: Uuid,
    ) -> Result<()> {
        sqlx::query("DELETE FROM gpg_public_keys WHERE id = $1 AND user_id = $2")
            .bind(key_id)
            .bind(user_id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// All GPG public keys belonging to anyone with access to a project (owner + members) --
    /// used to check a snapshot's signature under "vigilant mode" without needing to know exactly
    /// which member authored it (apich-vcs snapshots don't yet carry a real per-member author id).
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn list_gpg_public_keys_for_project(
        &self,
        project_id: Uuid,
    ) -> Result<Vec<GpgPublicKey>> {
        let keys = sqlx::query_as::<_, GpgPublicKey>(
            r"
            SELECT DISTINCT k.* FROM gpg_public_keys k
            WHERE k.user_id = (SELECT owner_id FROM projects WHERE id = $1)
               OR k.user_id IN (SELECT user_id FROM project_members WHERE project_id = $1)
            ",
        )
        .bind(project_id)
        .fetch_all(self.pool)
        .await?;
        Ok(keys)
    }

    /// Set project vigilant mode.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn set_project_vigilant_mode(
        &self,
        project_id: Uuid,
        enabled: bool,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE projects SET vigilant_mode = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2",
        )
        .bind(enabled)
        .bind(project_id)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    // --- System Settings & Invitations ---

    /// Get system settings.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn get_system_settings(&self) -> Result<SystemSettings> {
        let settings =
            sqlx::query_as::<_, SystemSettings>("SELECT * FROM system_settings WHERE id = 1")
                .fetch_one(self.pool)
                .await?;
        Ok(settings)
    }

    /// Update system settings.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn update_system_settings(
        &self,
        dto: UpdateSystemSettingsDto,
    ) -> Result<SystemSettings> {
        let settings = sqlx::query_as::<_, SystemSettings>(
            r"
            UPDATE system_settings
            SET registration_mode = COALESCE($1, registration_mode),
                smtp_host = COALESCE($2, smtp_host),
                smtp_port = COALESCE($3, smtp_port),
                smtp_username = COALESCE($4, smtp_username),
                smtp_password = COALESCE($5, smtp_password),
                smtp_from_email = COALESCE($6, smtp_from_email),
                smtp_from_name = COALESCE($7, smtp_from_name),
                smtp_use_tls = COALESCE($8, smtp_use_tls),
                smtp_force_tls = COALESCE($9, smtp_force_tls),
                smtp_enabled = COALESCE($10, smtp_enabled),
                require_2fa = COALESCE($11, require_2fa),
                updated_at = CURRENT_TIMESTAMP
            WHERE id = 1
            RETURNING *
            ",
        )
        .bind(dto.registration_mode)
        .bind(dto.smtp_host)
        .bind(dto.smtp_port)
        .bind(dto.smtp_username)
        .bind(dto.smtp_password)
        .bind(dto.smtp_from_email)
        .bind(dto.smtp_from_name)
        .bind(dto.smtp_use_tls)
        .bind(dto.smtp_force_tls)
        .bind(dto.smtp_enabled)
        .bind(dto.require_2fa)
        .fetch_one(self.pool)
        .await?;

        Ok(settings)
    }

    /// Update global 2FA policy enforcement.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn update_system_settings_2fa(&self, require_2fa: bool) -> Result<SystemSettings> {
        let settings = sqlx::query_as::<_, SystemSettings>(
            r"
            UPDATE system_settings
            SET require_2fa = $1,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = 1
            RETURNING *
            ",
        )
        .bind(require_2fa)
        .fetch_one(self.pool)
        .await?;
        Ok(settings)
    }

    /// Create invitation.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn create_invitation(
        &self,
        token: &str,
        dto: CreateInvitationDto,
    ) -> Result<Invitation> {
        let id = Uuid::now_v7();
        let role = dto.role.unwrap_or_else(|| "member".to_string());
        let max_uses = dto.max_uses.unwrap_or(1).max(1);

        let invite = sqlx::query_as::<_, Invitation>(
            r"
            INSERT INTO invitations (id, token, email, org_id, team_id, role, inviter_id, max_uses, used_count, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 0, $9)
            RETURNING *
            ",
        )
        .bind(id)
        .bind(token)
        .bind(&dto.email)
        .bind(dto.org_id)
        .bind(dto.team_id)
        .bind(role)
        .bind(dto.inviter_id)
        .bind(max_uses)
        .bind(dto.expires_at)
        .fetch_one(self.pool)
        .await?;

        Ok(invite)
    }

    /// Get invitation by token.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn get_invitation_by_token(
        &self,
        token: &str,
    ) -> Result<Option<Invitation>> {
        let invite = sqlx::query_as::<_, Invitation>(
            "SELECT * FROM invitations WHERE token = $1 AND used_count < max_uses AND expires_at > CURRENT_TIMESTAMP",
        )
        .bind(token)
        .fetch_optional(self.pool)
        .await?;
        Ok(invite)
    }

    /// Mark invitation used by incrementing `used_count` and updating `used_at`.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn mark_invitation_used(
        &self,
        token: &str,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE invitations SET used_count = used_count + 1, used_at = CURRENT_TIMESTAMP WHERE token = $1 AND used_count < max_uses",
        )
        .bind(token)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// List all invitations ordered by creation date.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn list_invitations(&self) -> Result<Vec<Invitation>> {
        let list = sqlx::query_as::<_, Invitation>(
            "SELECT * FROM invitations ORDER BY created_at DESC",
        )
        .fetch_all(self.pool)
        .await?;
        Ok(list)
    }

    /// Delete or revoke an invitation by id.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn delete_invitation(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM invitations WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    // --- SSO / OAuth2 Platform Operations ---

    /// Create oauth client.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn create_oauth_client(
        &self,
        dto: CreateOAuthClientDto,
    ) -> Result<OAuthClient> {
        let client = sqlx::query_as::<_, OAuthClient>(
            r"
            INSERT INTO oauth_clients (client_id, client_secret_hash, name, redirect_uris, is_confidential)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            ",
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

    /// Get oauth client by ID.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn get_oauth_client_by_id(
        &self,
        client_id: &str,
    ) -> Result<Option<OAuthClient>> {
        let client =
            sqlx::query_as::<_, OAuthClient>("SELECT * FROM oauth_clients WHERE client_id = $1")
                .bind(client_id)
                .fetch_optional(self.pool)
                .await?;
        Ok(client)
    }

    /// List oauth clients.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn list_oauth_clients(&self) -> Result<Vec<OAuthClient>> {
        let clients = sqlx::query_as::<_, OAuthClient>(
            "SELECT * FROM oauth_clients ORDER BY created_at DESC",
        )
        .fetch_all(self.pool)
        .await?;
        Ok(clients)
    }

    /// Create oauth auth code.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
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
            r"
            INSERT INTO oauth_auth_codes (code, client_id, user_id, redirect_uri, scope, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            ",
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

    /// Consume oauth auth code.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn consume_oauth_auth_code(
        &self,
        code: &str,
    ) -> Result<Option<OAuthAuthCode>> {
        let auth_code = sqlx::query_as::<_, OAuthAuthCode>(
            r"
            DELETE FROM oauth_auth_codes
            WHERE code = $1 AND expires_at > CURRENT_TIMESTAMP
            RETURNING *
            ",
        )
        .bind(code)
        .fetch_optional(self.pool)
        .await?;

        Ok(auth_code)
    }

    // --- Template Library Operations ---

    /// Create template.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn create_template(
        &self,
        dto: CreateTemplateDto,
    ) -> Result<Template> {
        let id = Uuid::now_v7();
        let template = sqlx::query_as::<_, Template>(
            r"
            INSERT INTO templates (id, kind, name, slug, description, owner_user_id, visibility)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            ",
        )
        .bind(id)
        .bind(&dto.kind)
        .bind(&dto.name)
        .bind(&dto.slug)
        .bind(&dto.description)
        .bind(dto.owner_user_id)
        .bind(&dto.visibility)
        .fetch_one(self.pool)
        .await?;
        Ok(template)
    }

    /// Get template by ID.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn get_template_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<Template>> {
        let template = sqlx::query_as::<_, Template>("SELECT * FROM templates WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool)
            .await?;
        Ok(template)
    }

    /// Every template a `user_id` may see: their own (any visibility), plus every 'public' one,
    /// plus every 'shared' one whose `template_shares` list includes an org/team they belong to.
    /// `kind` optionally narrows the gallery to one content type (see `TemplateKind`).
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn list_visible_templates(
        &self,
        user_id: Uuid,
        kind: Option<&str>,
    ) -> Result<Vec<TemplateWithLatestVersion>> {
        let templates = sqlx::query_as::<_, TemplateWithLatestVersion>(
            r"
            SELECT
                t.id, t.kind, t.name, t.slug, t.description, t.owner_user_id, t.visibility,
                t.created_at, t.updated_at,
                u.username AS owner_username,
                COUNT(tv.id) AS version_count,
                (ARRAY_AGG(tv.version_label ORDER BY tv.created_at DESC))[1] AS latest_version_label,
                (ARRAY_AGG(tv.id ORDER BY tv.created_at DESC))[1] AS latest_version_id
            FROM templates t
            JOIN users u ON u.id = t.owner_user_id
            LEFT JOIN template_versions tv ON tv.template_id = t.id
            WHERE ($2::VARCHAR IS NULL OR t.kind = $2)
              AND (
                t.owner_user_id = $1
                OR t.visibility = 'public'
                OR (t.visibility = 'shared' AND EXISTS (
                    SELECT 1 FROM template_shares ts
                    LEFT JOIN org_members om ON ts.org_id = om.org_id AND om.user_id = $1
                    LEFT JOIN team_members tm ON ts.team_id = tm.team_id AND tm.user_id = $1
                    WHERE ts.template_id = t.id AND (om.user_id IS NOT NULL OR tm.user_id IS NOT NULL)
                ))
              )
            GROUP BY t.id, u.username
            ORDER BY t.updated_at DESC
            ",
        )
        .bind(user_id)
        .bind(kind)
        .fetch_all(self.pool)
        .await?;
        Ok(templates)
    }

    /// List templates owned by.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn list_templates_owned_by(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<Template>> {
        let templates = sqlx::query_as::<_, Template>(
            "SELECT * FROM templates WHERE owner_user_id = $1 ORDER BY updated_at DESC",
        )
        .bind(user_id)
        .fetch_all(self.pool)
        .await?;
        Ok(templates)
    }

    /// Update template visibility.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn update_template_visibility(
        &self,
        id: Uuid,
        visibility: &str,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE templates SET visibility = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2",
        )
        .bind(visibility)
        .bind(id)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Delete template.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn delete_template(
        &self,
        id: Uuid,
    ) -> Result<()> {
        sqlx::query("DELETE FROM templates WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// Add template share.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn add_template_share(
        &self,
        template_id: Uuid,
        org_id: Option<Uuid>,
        team_id: Option<Uuid>,
    ) -> Result<TemplateShare> {
        let id = Uuid::now_v7();
        let share = sqlx::query_as::<_, TemplateShare>(
            r"
            INSERT INTO template_shares (id, template_id, org_id, team_id)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            ",
        )
        .bind(id)
        .bind(template_id)
        .bind(org_id)
        .bind(team_id)
        .fetch_one(self.pool)
        .await?;
        Ok(share)
    }

    /// Remove template share.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn remove_template_share(
        &self,
        share_id: Uuid,
    ) -> Result<()> {
        sqlx::query("DELETE FROM template_shares WHERE id = $1")
            .bind(share_id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// List template shares.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn list_template_shares(
        &self,
        template_id: Uuid,
    ) -> Result<Vec<TemplateShare>> {
        let shares = sqlx::query_as::<_, TemplateShare>(
            "SELECT * FROM template_shares WHERE template_id = $1 ORDER BY created_at ASC",
        )
        .bind(template_id)
        .fetch_all(self.pool)
        .await?;
        Ok(shares)
    }

    /// Publish template version.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn publish_template_version(
        &self,
        dto: PublishTemplateVersionDto,
    ) -> Result<TemplateVersion> {
        let id = Uuid::now_v7();
        let version = sqlx::query_as::<_, TemplateVersion>(
            r"
            INSERT INTO template_versions (id, template_id, version_label, changelog, content, published_by)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            ",
        )
        .bind(id)
        .bind(dto.template_id)
        .bind(&dto.version_label)
        .bind(&dto.changelog)
        .bind(&dto.content)
        .bind(dto.published_by)
        .fetch_one(self.pool)
        .await?;

        sqlx::query("UPDATE templates SET updated_at = CURRENT_TIMESTAMP WHERE id = $1")
            .bind(dto.template_id)
            .execute(self.pool)
            .await?;

        Ok(version)
    }

    /// List template versions.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn list_template_versions(
        &self,
        template_id: Uuid,
    ) -> Result<Vec<TemplateVersion>> {
        let versions = sqlx::query_as::<_, TemplateVersion>(
            "SELECT * FROM template_versions WHERE template_id = $1 ORDER BY created_at DESC",
        )
        .bind(template_id)
        .fetch_all(self.pool)
        .await?;
        Ok(versions)
    }

    /// Get template version by ID.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn get_template_version_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<TemplateVersion>> {
        let version =
            sqlx::query_as::<_, TemplateVersion>("SELECT * FROM template_versions WHERE id = $1")
                .bind(id)
                .fetch_optional(self.pool)
                .await?;
        Ok(version)
    }

    /// Get latest template version.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn get_latest_template_version(
        &self,
        template_id: Uuid,
    ) -> Result<Option<TemplateVersion>> {
        let version = sqlx::query_as::<_, TemplateVersion>(
            "SELECT * FROM template_versions WHERE template_id = $1 ORDER BY created_at DESC LIMIT 1",
        )
        .bind(template_id)
        .fetch_optional(self.pool)
        .await?;
        Ok(version)
    }

    /// Get a user's TOTP secret.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn get_user_totp_secret(&self, user_id: Uuid) -> Result<Option<String>> {
        let rec: Option<(Option<String>,)> =
            sqlx::query_as("SELECT totp_secret FROM users WHERE id = $1")
                .bind(user_id)
                .fetch_optional(self.pool)
                .await?;
        Ok(rec.and_then(|(s,)| s))
    }

    /// Set a user's TOTP secret and enabled status.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn set_user_totp(
        &self,
        user_id: Uuid,
        secret: Option<&str>,
        enabled: bool,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE users SET totp_secret = $2, totp_enabled = $3, updated_at = CURRENT_TIMESTAMP WHERE id = $1",
        )
        .bind(user_id)
        .bind(secret)
        .bind(enabled)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Create a transient 2FA challenge for a user during login.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn create_2fa_challenge(
        &self,
        user_id: Uuid,
        return_to: Option<&str>,
    ) -> Result<User2faChallenge> {
        let expires_at = Utc::now() + chrono::Duration::minutes(10);
        let challenge = sqlx::query_as::<_, User2faChallenge>(
            r"
            INSERT INTO user_2fa_challenges (user_id, return_to, expires_at)
            VALUES ($1, $2, $3)
            RETURNING *
            ",
        )
        .bind(user_id)
        .bind(return_to)
        .bind(expires_at)
        .fetch_one(self.pool)
        .await?;
        Ok(challenge)
    }

    /// Get a 2FA challenge by ID, ensuring it has not expired.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn get_2fa_challenge(
        &self,
        challenge_id: Uuid,
    ) -> Result<Option<User2faChallenge>> {
        let challenge = sqlx::query_as::<_, User2faChallenge>(
            "SELECT * FROM user_2fa_challenges WHERE id = $1 AND expires_at > CURRENT_TIMESTAMP",
        )
        .bind(challenge_id)
        .fetch_optional(self.pool)
        .await?;
        Ok(challenge)
    }

    /// Set an email verification code hash on a 2FA challenge.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn set_2fa_challenge_email_code(
        &self,
        challenge_id: Uuid,
        code_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<()> {
        sqlx::query(
            r"
            UPDATE user_2fa_challenges
            SET email_code_hash = $2, email_code_expires_at = $3
            WHERE id = $1
            ",
        )
        .bind(challenge_id)
        .bind(code_hash)
        .bind(expires_at)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Delete a 2FA challenge once completed.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn delete_2fa_challenge(&self, challenge_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM user_2fa_challenges WHERE id = $1")
            .bind(challenge_id)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// Clean up expired 2FA challenges.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn cleanup_expired_2fa_challenges(&self) -> Result<u64> {
        let result =
            sqlx::query("DELETE FROM user_2fa_challenges WHERE expires_at <= CURRENT_TIMESTAMP")
                .execute(self.pool)
                .await?;
        Ok(result.rows_affected())
    }

    /// Retrieve stored Git credential for a user and provider (e.g. "github").
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn get_user_git_credential(
        &self,
        user_id: Uuid,
        provider: &str,
    ) -> Result<Option<UserGitCredential>> {
        let cred = sqlx::query_as::<_, UserGitCredential>(
            r"
            SELECT * FROM user_git_credentials
            WHERE user_id = $1 AND provider = $2
            ",
        )
        .bind(user_id)
        .bind(provider)
        .fetch_optional(self.pool)
        .await?;
        Ok(cred)
    }

    /// Upsert a Git credential for a user and provider.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn upsert_user_git_credential(
        &self,
        user_id: Uuid,
        dto: &UpsertGitCredentialDto,
    ) -> Result<UserGitCredential> {
        let cred = sqlx::query_as::<_, UserGitCredential>(
            r"
            INSERT INTO user_git_credentials (user_id, provider, account_username, access_token, updated_at)
            VALUES ($1, $2, $3, $4, CURRENT_TIMESTAMP)
            ON CONFLICT (user_id, provider)
            DO UPDATE SET
                account_username = EXCLUDED.account_username,
                access_token = EXCLUDED.access_token,
                updated_at = CURRENT_TIMESTAMP
            RETURNING *;
            ",
        )
        .bind(user_id)
        .bind(&dto.provider)
        .bind(&dto.account_username)
        .bind(&dto.access_token)
        .fetch_one(self.pool)
        .await?;
        Ok(cred)
    }

    /// Delete a Git credential for a user and provider.
    ///
    /// # Errors
    /// Returns an error if the database query or operation fails.
    pub async fn delete_user_git_credential(
        &self,
        user_id: Uuid,
        provider: &str,
    ) -> Result<bool> {
        let res = sqlx::query(
            "DELETE FROM user_git_credentials WHERE user_id = $1 AND provider = $2",
        )
        .bind(user_id)
        .bind(provider)
        .execute(self.pool)
        .await?;
        Ok(res.rows_affected() > 0)
    }
}
