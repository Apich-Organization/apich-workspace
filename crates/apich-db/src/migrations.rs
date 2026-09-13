use crate::error::Result;
use chrono::DateTime;
use chrono::Utc;
use sqlx::PgPool;
use tracing::info;

/// Representation of a database schema migration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Migration {
    /// Unique sequential version number
    pub version: i32,
    /// Human-readable migration name
    pub name: &'static str,
    /// SQL script for the migration
    pub sql: &'static str,
}

/// Execution result for an applied migration
#[derive(Debug, Clone)]
pub struct MigrationResult {
    /// Schema version number.
    pub version: i32,
    /// Human-readable migration name or description.
    pub name: String,
    /// Timestamp when the migration was applied.
    pub applied_at: DateTime<Utc>,
}

/// Extensible Migration Registry & Runner.
///
/// Other modules (such as `apich-git`, `apich-slide`, plugins) can dynamically register
/// custom migrations to evolve the database schema cleanly.
#[derive(Debug, Clone)]
pub struct MigrationManager {
    migrations: Vec<Migration>,
}

impl Default for MigrationManager {
    fn default() -> Self {
        Self::new()
    }
}

impl MigrationManager {
    /// Create a migration manager with default APICH core migrations
    #[must_use]
    pub fn new() -> Self {
        let mut manager = Self::empty();
        manager.register(Migration {
            version: 1,
            name: "001_core_schema_pg18",
            sql: CORE_SCHEMA_PG18_SQL,
        });
        manager.register(Migration {
            version: 2,
            name: "002_row_level_security",
            sql: ROW_LEVEL_SECURITY_SQL,
        });
        manager.register(Migration {
            version: 3,
            name: "003_identity_projects_sso",
            sql: IDENTITY_PROJECTS_SSO_SQL,
        });
        manager.register(Migration {
            version: 4,
            name: "004_hub_integrations_and_tables",
            sql: HUB_INTEGRATIONS_AND_TABLES_SQL,
        });
        manager.register(Migration {
            version: 5,
            name: "005_sandbox_idle_tracking",
            sql: SANDBOX_IDLE_TRACKING_SQL,
        });
        manager.register(Migration {
            version: 6,
            name: "006_pat_ssh_gpg_keys",
            sql: PAT_SSH_GPG_KEYS_SQL,
        });
        manager.register(Migration {
            version: 7,
            name: "007_template_library",
            sql: TEMPLATE_LIBRARY_SQL,
        });
        manager.register(Migration {
            version: 8,
            name: "008_invitation_codes_enhancement",
            sql: INVITATION_CODES_ENHANCEMENT_SQL,
        });
        manager.register(Migration {
            version: 9,
            name: "009_passkey_json_support",
            sql: PASSKEY_JSON_SUPPORT_SQL,
        });
        manager
    }

    /// Create an empty migration manager without default migrations
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            migrations: Vec::new(),
        }
    }

    /// Register a new migration. Future modules or plugins use this to extend the DB schema.
    pub fn register(
        &mut self,
        migration: Migration,
    ) -> &mut Self {
        // Prevent duplicate version numbers
        self.migrations.retain(|m| m.version != migration.version);
        self.migrations.push(migration);
        self.migrations.sort_by_key(|m| m.version);
        self
    }

    /// Register multiple migrations at once
    pub fn register_all<I: IntoIterator<Item = Migration>>(
        &mut self,
        iter: I,
    ) -> &mut Self {
        for migration in iter {
            self.register(migration);
        }
        self
    }

    /// List all currently registered migrations
    #[must_use]
    pub fn migrations(&self) -> &[Migration] {
        &self.migrations
    }

    /// Run all pending migrations against the PostgreSQL pool within atomic transactions
    ///
    /// # Errors
    /// Returns an error if querying or executing database migration statements fails.
    pub async fn migrate(
        &self,
        pool: &PgPool,
    ) -> Result<Vec<MigrationResult>> {
        info!("Running database migrations for APICH Workspace");

        // 1. Ensure migrations tracking table exists
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS _schema_migrations (
                version INT PRIMARY KEY,
                name VARCHAR(128) NOT NULL,
                applied_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            ",
        )
        .execute(pool)
        .await?;

        // 2. Fetch already applied versions
        let applied_versions: Vec<i32> =
            sqlx::query_scalar("SELECT version FROM _schema_migrations ORDER BY version ASC;")
                .fetch_all(pool)
                .await?;

        let mut newly_applied = Vec::new();

        // 3. Execute pending migrations in sequential order
        for migration in &self.migrations {
            if applied_versions.contains(&migration.version) {
                continue;
            }

            info!(
                version = migration.version,
                name = migration.name,
                "Applying pending migration"
            );

            let mut tx = pool.begin().await?;

            sqlx::raw_sql(migration.sql).execute(&mut *tx).await?;

            let applied_at: DateTime<Utc> = sqlx::query_scalar(
                r"
                INSERT INTO _schema_migrations (version, name, applied_at)
                VALUES ($1, $2, CURRENT_TIMESTAMP)
                RETURNING applied_at;
                ",
            )
            .bind(migration.version)
            .bind(migration.name)
            .fetch_one(&mut *tx)
            .await?;

            tx.commit().await?;

            newly_applied.push(MigrationResult {
                version: migration.version,
                name: migration.name.to_string(),
                applied_at,
            });

            info!(
                version = migration.version,
                name = migration.name,
                "Migration applied successfully"
            );
        }

        Ok(newly_applied)
    }
}

/// Convenience function to run all default migrations
///
/// # Errors
/// Returns an error if executing pending migrations fails.
pub async fn run_migrations(pool: &PgPool) -> Result<()> {
    MigrationManager::new().migrate(pool).await?;
    Ok(())
}

/// Migration 001: Core Schema with PostgreSQL 18 native features:
/// - `UUIDv7` default generation
/// - Generated tsvector with GIN index for Full-Text Search
/// - GIN `jsonb_path_ops` indexes for JSONB metadata
/// - Core tables: users, workspaces, `workspace_members`, documents, knowledge nodes/edges, audit logs
pub const CORE_SCHEMA_PG18_SQL: &str = r"
-- 1. Custom Types
DO $$ BEGIN
    CREATE TYPE user_role AS ENUM ('admin', 'member', 'guest');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
    CREATE TYPE workspace_visibility AS ENUM ('private', 'internal', 'public');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
    CREATE TYPE member_role AS ENUM ('owner', 'maintainer', 'contributor', 'viewer');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
    CREATE TYPE doc_type AS ENUM ('markdown', 'typst', 'latex', 'sqlitetable', 'slide', 'code', 'other');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- 2. Updated_at trigger function
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ language 'plpgsql';

-- 3. Users table (PG18 uuidv7)
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    username VARCHAR(64) UNIQUE NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    display_name VARCHAR(128) NOT NULL,
    avatar_url TEXT,
    role user_role NOT NULL DEFAULT 'member',
    storage_quota_bytes BIGINT NOT NULL DEFAULT 10737418240,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 4. Workspaces table
CREATE TABLE IF NOT EXISTS workspaces (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    slug VARCHAR(64) NOT NULL,
    name VARCHAR(128) NOT NULL,
    description TEXT,
    visibility workspace_visibility NOT NULL DEFAULT 'private',
    container_name VARCHAR(128),
    storage_path TEXT,
    settings JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT uq_owner_slug UNIQUE (owner_id, slug)
);

-- 5. Workspace Members table
CREATE TABLE IF NOT EXISTS workspace_members (
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role member_role NOT NULL DEFAULT 'viewer',
    joined_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (workspace_id, user_id)
);

-- 6. Documents table with PG18 Generated tsvector & GIN indexing
CREATE TABLE IF NOT EXISTS documents (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    rel_path VARCHAR(512) NOT NULL,
    title VARCHAR(255) NOT NULL,
    content TEXT NOT NULL DEFAULT '',
    doc_type doc_type NOT NULL DEFAULT 'markdown',
    version INT NOT NULL DEFAULT 1,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    search_tokens tsvector GENERATED ALWAYS AS (to_tsvector('english', coalesce(title, '') || ' ' || coalesce(content, ''))) STORED,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT uq_workspace_rel_path UNIQUE (workspace_id, rel_path)
);

-- 7. Knowledge Nodes (Section 4: Knowledge Graph)
CREATE TABLE IF NOT EXISTS knowledge_nodes (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    node_type VARCHAR(64) NOT NULL,
    title VARCHAR(255) NOT NULL,
    content_hash VARCHAR(64),
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 8. Knowledge Edges
CREATE TABLE IF NOT EXISTS knowledge_edges (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    source_id UUID NOT NULL REFERENCES knowledge_nodes(id) ON DELETE CASCADE,
    target_id UUID NOT NULL REFERENCES knowledge_nodes(id) ON DELETE CASCADE,
    relation_type VARCHAR(64) NOT NULL,
    weight REAL NOT NULL DEFAULT 1.0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT uq_edge UNIQUE (workspace_id, source_id, target_id, relation_type)
);

-- 9. Audit Logs (Section 6: Version Control & Audit Trail)
CREATE TABLE IF NOT EXISTS audit_logs (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    workspace_id UUID REFERENCES workspaces(id) ON DELETE CASCADE,
    action VARCHAR(128) NOT NULL,
    details JSONB NOT NULL DEFAULT '{}'::jsonb,
    ip_address VARCHAR(45),
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 10. Indexes
CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_workspaces_owner ON workspaces(owner_id);
CREATE INDEX IF NOT EXISTS idx_documents_workspace ON documents(workspace_id);
CREATE INDEX IF NOT EXISTS idx_knowledge_nodes_workspace ON knowledge_nodes(workspace_id);
CREATE INDEX IF NOT EXISTS idx_knowledge_edges_workspace ON knowledge_edges(workspace_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_workspace ON audit_logs(workspace_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_created_at ON audit_logs(created_at);

-- Modern High-Performance GIN Indexes
CREATE INDEX IF NOT EXISTS idx_documents_search ON documents USING GIN(search_tokens);
CREATE INDEX IF NOT EXISTS idx_documents_metadata ON documents USING GIN(metadata jsonb_path_ops);
CREATE INDEX IF NOT EXISTS idx_knowledge_nodes_metadata ON knowledge_nodes USING GIN(metadata jsonb_path_ops);
CREATE INDEX IF NOT EXISTS idx_workspaces_settings ON workspaces USING GIN(settings jsonb_path_ops);

-- Triggers
DROP TRIGGER IF EXISTS trg_users_updated_at ON users;
CREATE TRIGGER trg_users_updated_at BEFORE UPDATE ON users FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

DROP TRIGGER IF EXISTS trg_workspaces_updated_at ON workspaces;
CREATE TRIGGER trg_workspaces_updated_at BEFORE UPDATE ON workspaces FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

DROP TRIGGER IF EXISTS trg_documents_updated_at ON documents;
CREATE TRIGGER trg_documents_updated_at BEFORE UPDATE ON documents FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

DROP TRIGGER IF EXISTS trg_knowledge_nodes_updated_at ON knowledge_nodes;
CREATE TRIGGER trg_knowledge_nodes_updated_at BEFORE UPDATE ON knowledge_nodes FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
";

/// Migration 002: Row-Level Security (RLS) for multi-tenant workspace isolation.
pub const ROW_LEVEL_SECURITY_SQL: &str = r"
-- Helper function with SECURITY DEFINER to evaluate workspace access
-- without triggering recursive RLS execution
CREATE OR REPLACE FUNCTION can_access_workspace(ws_id UUID, u_id UUID)
RETURNS BOOLEAN AS $$
BEGIN
    IF ws_id IS NULL THEN
        RETURN FALSE;
    END IF;
    RETURN EXISTS (
        SELECT 1 FROM workspaces
        WHERE id = ws_id AND (
            visibility = 'public'
            OR (u_id IS NOT NULL AND (
                owner_id = u_id
                OR EXISTS (
                    SELECT 1 FROM workspace_members
                    WHERE workspace_id = ws_id AND user_id = u_id
                )
            ))
        )
    );
END;
$$ LANGUAGE plpgsql STABLE SECURITY DEFINER;

-- Enable Row Level Security on core multi-tenant tables
ALTER TABLE workspaces ENABLE ROW LEVEL SECURITY;
ALTER TABLE workspace_members ENABLE ROW LEVEL SECURITY;
ALTER TABLE documents ENABLE ROW LEVEL SECURITY;
ALTER TABLE knowledge_nodes ENABLE ROW LEVEL SECURITY;
ALTER TABLE knowledge_edges ENABLE ROW LEVEL SECURITY;
ALTER TABLE audit_logs ENABLE ROW LEVEL SECURITY;

-- 1. Workspace Policies
DROP POLICY IF EXISTS workspace_select_policy ON workspaces;
CREATE POLICY workspace_select_policy ON workspaces
    AS PERMISSIVE
    FOR SELECT
    TO PUBLIC
    USING (
        current_setting('app.bypass_rls', true) = 'on'
        OR can_access_workspace(id, NULLIF(current_setting('app.current_user_id', true), '')::UUID)
    );

DROP POLICY IF EXISTS workspace_insert_policy ON workspaces;
CREATE POLICY workspace_insert_policy ON workspaces
    AS PERMISSIVE
    FOR INSERT
    TO PUBLIC
    WITH CHECK (
        current_setting('app.bypass_rls', true) = 'on'
        OR current_setting('app.current_user_id', true) IS NULL
        OR current_setting('app.current_user_id', true) = ''
        OR owner_id = NULLIF(current_setting('app.current_user_id', true), '')::UUID
    );

DROP POLICY IF EXISTS workspace_modify_policy ON workspaces;
CREATE POLICY workspace_modify_policy ON workspaces
    AS PERMISSIVE
    FOR ALL
    TO PUBLIC
    USING (
        current_setting('app.bypass_rls', true) = 'on'
        OR current_setting('app.current_user_id', true) IS NULL
        OR current_setting('app.current_user_id', true) = ''
        OR can_access_workspace(id, NULLIF(current_setting('app.current_user_id', true), '')::UUID)
    );

-- 2. Workspace Members Policy
DROP POLICY IF EXISTS workspace_members_policy ON workspace_members;
CREATE POLICY workspace_members_policy ON workspace_members
    AS PERMISSIVE
    FOR ALL
    TO PUBLIC
    USING (
        current_setting('app.bypass_rls', true) = 'on'
        OR current_setting('app.current_user_id', true) IS NULL
        OR current_setting('app.current_user_id', true) = ''
        OR user_id = NULLIF(current_setting('app.current_user_id', true), '')::UUID
        OR can_access_workspace(workspace_id, NULLIF(current_setting('app.current_user_id', true), '')::UUID)
    );

-- 3. Document Policies
DROP POLICY IF EXISTS document_select_policy ON documents;
CREATE POLICY document_select_policy ON documents
    AS PERMISSIVE
    FOR SELECT
    TO PUBLIC
    USING (
        current_setting('app.bypass_rls', true) = 'on'
        OR can_access_workspace(workspace_id, NULLIF(current_setting('app.current_user_id', true), '')::UUID)
    );

DROP POLICY IF EXISTS document_modify_policy ON documents;
CREATE POLICY document_modify_policy ON documents
    AS PERMISSIVE
    FOR ALL
    TO PUBLIC
    USING (
        current_setting('app.bypass_rls', true) = 'on'
        OR current_setting('app.current_user_id', true) IS NULL
        OR current_setting('app.current_user_id', true) = ''
        OR can_access_workspace(workspace_id, NULLIF(current_setting('app.current_user_id', true), '')::UUID)
    );

-- 4. Knowledge Nodes Policy
DROP POLICY IF EXISTS knowledge_nodes_policy ON knowledge_nodes;
CREATE POLICY knowledge_nodes_policy ON knowledge_nodes
    AS PERMISSIVE
    FOR ALL
    TO PUBLIC
    USING (
        current_setting('app.bypass_rls', true) = 'on'
        OR current_setting('app.current_user_id', true) IS NULL
        OR current_setting('app.current_user_id', true) = ''
        OR can_access_workspace(workspace_id, NULLIF(current_setting('app.current_user_id', true), '')::UUID)
    );

-- 5. Knowledge Edges Policy
DROP POLICY IF EXISTS knowledge_edges_policy ON knowledge_edges;
CREATE POLICY knowledge_edges_policy ON knowledge_edges
    AS PERMISSIVE
    FOR ALL
    TO PUBLIC
    USING (
        current_setting('app.bypass_rls', true) = 'on'
        OR current_setting('app.current_user_id', true) IS NULL
        OR current_setting('app.current_user_id', true) = ''
        OR can_access_workspace(workspace_id, NULLIF(current_setting('app.current_user_id', true), '')::UUID)
    );

-- 6. Audit Logs Policy
DROP POLICY IF EXISTS audit_logs_policy ON audit_logs;
CREATE POLICY audit_logs_policy ON audit_logs
    AS PERMISSIVE
    FOR ALL
    TO PUBLIC
    USING (
        current_setting('app.bypass_rls', true) = 'on'
        OR current_setting('app.current_user_id', true) IS NULL
        OR current_setting('app.current_user_id', true) = ''
        OR user_id = NULLIF(current_setting('app.current_user_id', true), '')::UUID
        OR can_access_workspace(workspace_id, NULLIF(current_setting('app.current_user_id', true), '')::UUID)
    );
";

/// Migration 003: Identity Tree, Hierarchical Permissions, Projects, FIDO2/Passkeys, Sessions, SSO, and Settings
pub const IDENTITY_PROJECTS_SSO_SQL: &str = r"
-- 1. Ensure users table has is_platform_admin
ALTER TABLE users ADD COLUMN IF NOT EXISTS is_platform_admin BOOLEAN NOT NULL DEFAULT false;

-- 2. Organizations
CREATE TABLE IF NOT EXISTS organizations (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    slug VARCHAR(64) UNIQUE NOT NULL,
    name VARCHAR(128) NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 3. Organization Memberships
CREATE TABLE IF NOT EXISTS org_members (
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role VARCHAR(32) NOT NULL DEFAULT 'member', -- 'owner', 'admin', 'member', 'guest'
    joined_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (org_id, user_id)
);

-- 4. Teams (Recursive tree with parent_team_id)
CREATE TABLE IF NOT EXISTS teams (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    parent_team_id UUID REFERENCES teams(id) ON DELETE CASCADE,
    name VARCHAR(128) NOT NULL,
    slug VARCHAR(64) NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT uq_team_org_slug UNIQUE (org_id, slug)
);

-- 5. Team Memberships
CREATE TABLE IF NOT EXISTS team_members (
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role VARCHAR(32) NOT NULL DEFAULT 'member', -- 'admin', 'maintainer', 'member', 'viewer'
    joined_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (team_id, user_id)
);

-- 6. Projects (The core unit of APICH workspace)
CREATE TABLE IF NOT EXISTS projects (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    org_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    team_id UUID REFERENCES teams(id) ON DELETE SET NULL,
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    name VARCHAR(128) NOT NULL,
    slug VARCHAR(64) NOT NULL,
    description TEXT,
    storage_path TEXT NOT NULL,
    status VARCHAR(32) NOT NULL DEFAULT 'active', -- 'active', 'paused', 'archived', 'deleted'
    vcs_initialized BOOLEAN NOT NULL DEFAULT false,
    settings JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT uq_project_org_slug UNIQUE (org_id, slug)
);

-- 7. Project Sandboxes (1 Project + 1 User container mapping)
CREATE TABLE IF NOT EXISTS project_sandboxes (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    container_name VARCHAR(128) NOT NULL,
    status VARCHAR(32) NOT NULL DEFAULT 'stopped', -- 'running', 'stopped', 'terminated'
    last_started_at TIMESTAMPTZ,
    last_stopped_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT uq_sandbox_project_user UNIQUE (project_id, user_id)
);

-- 8. FIDO2 / WebAuthn Passkey Credentials
CREATE TABLE IF NOT EXISTS fido2_credentials (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    credential_id TEXT NOT NULL UNIQUE,
    public_key BYTEA NOT NULL,
    counter BIGINT NOT NULL DEFAULT 0,
    device_name VARCHAR(128) NOT NULL DEFAULT 'Passkey',
    aaguid BYTEA,
    passkey_json TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_used_at TIMESTAMPTZ
);

-- 9. User Sessions
CREATE TABLE IF NOT EXISTS user_sessions (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash VARCHAR(128) NOT NULL UNIQUE,
    user_agent TEXT,
    ip_address VARCHAR(45),
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 10. OAuth2 / OIDC SSO Platform Clients & Authorization Codes
CREATE TABLE IF NOT EXISTS oauth_clients (
    client_id VARCHAR(64) PRIMARY KEY,
    client_secret_hash VARCHAR(255),
    name VARCHAR(128) NOT NULL,
    redirect_uris TEXT[] NOT NULL,
    is_confidential BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS oauth_auth_codes (
    code VARCHAR(128) PRIMARY KEY,
    client_id VARCHAR(64) NOT NULL REFERENCES oauth_clients(client_id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    redirect_uri TEXT NOT NULL,
    scope TEXT NOT NULL DEFAULT 'openid profile email',
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 11. System Settings & SMTP Configuration
CREATE TABLE IF NOT EXISTS system_settings (
    id INT PRIMARY KEY DEFAULT 1,
    registration_mode VARCHAR(32) NOT NULL DEFAULT 'invite_only', -- 'open', 'invite_only', 'admin_only'
    smtp_host VARCHAR(255),
    smtp_port INT,
    smtp_username VARCHAR(255),
    smtp_password VARCHAR(255),
    smtp_from_email VARCHAR(255),
    smtp_from_name VARCHAR(255),
    smtp_use_tls BOOLEAN NOT NULL DEFAULT true,
    smtp_enabled BOOLEAN NOT NULL DEFAULT false,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT single_row_settings CHECK (id = 1)
);

-- Ensure initial row exists
INSERT INTO system_settings (id, registration_mode)
VALUES (1, 'invite_only')
ON CONFLICT (id) DO NOTHING;

-- 12. Invitations
CREATE TABLE IF NOT EXISTS invitations (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    token VARCHAR(128) UNIQUE NOT NULL,
    email VARCHAR(255),
    org_id UUID REFERENCES organizations(id) ON DELETE CASCADE,
    team_id UUID REFERENCES teams(id) ON DELETE CASCADE,
    role VARCHAR(32) NOT NULL DEFAULT 'member',
    inviter_id UUID REFERENCES users(id) ON DELETE SET NULL,
    max_uses INT NOT NULL DEFAULT 1,
    used_count INT NOT NULL DEFAULT 0,
    expires_at TIMESTAMPTZ NOT NULL,
    used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 13. Indexes for high performance lookup
CREATE INDEX IF NOT EXISTS idx_teams_org ON teams(org_id);
CREATE INDEX IF NOT EXISTS idx_teams_parent ON teams(parent_team_id);
CREATE INDEX IF NOT EXISTS idx_team_members_user ON team_members(user_id);
CREATE INDEX IF NOT EXISTS idx_org_members_user ON org_members(user_id);
CREATE INDEX IF NOT EXISTS idx_projects_org ON projects(org_id);
CREATE INDEX IF NOT EXISTS idx_projects_team ON projects(team_id);
CREATE INDEX IF NOT EXISTS idx_projects_owner ON projects(owner_id);
CREATE INDEX IF NOT EXISTS idx_project_sandboxes_user ON project_sandboxes(user_id);
CREATE INDEX IF NOT EXISTS idx_fido2_user ON fido2_credentials(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_token ON user_sessions(token_hash);
CREATE INDEX IF NOT EXISTS idx_sessions_user ON user_sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_invitations_token ON invitations(token);
CREATE INDEX IF NOT EXISTS idx_invitations_email ON invitations(email);

-- Triggers for updated_at
DROP TRIGGER IF EXISTS trg_organizations_updated_at ON organizations;
CREATE TRIGGER trg_organizations_updated_at BEFORE UPDATE ON organizations FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

DROP TRIGGER IF EXISTS trg_teams_updated_at ON teams;
CREATE TRIGGER trg_teams_updated_at BEFORE UPDATE ON teams FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

DROP TRIGGER IF EXISTS trg_projects_updated_at ON projects;
CREATE TRIGGER trg_projects_updated_at BEFORE UPDATE ON projects FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

DROP TRIGGER IF EXISTS trg_project_sandboxes_updated_at ON project_sandboxes;
CREATE TRIGGER trg_project_sandboxes_updated_at BEFORE UPDATE ON project_sandboxes FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- 14. Project Members & Direct Sharing
CREATE TABLE IF NOT EXISTS project_members (
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role VARCHAR(32) NOT NULL DEFAULT 'editor', -- 'owner', 'admin', 'editor', 'viewer'
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (project_id, user_id)
);
CREATE INDEX IF NOT EXISTS idx_project_members_user ON project_members(user_id);
CREATE INDEX IF NOT EXISTS idx_project_members_proj ON project_members(project_id);

DROP TRIGGER IF EXISTS trg_project_members_updated_at ON project_members;
CREATE TRIGGER trg_project_members_updated_at BEFORE UPDATE ON project_members FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
";

/// SQL schema migration 004: External hub integrations and team override rules.
pub const HUB_INTEGRATIONS_AND_TABLES_SQL: &str = r"
-- 004: External Hub Integrations and Team Override Rules
ALTER TABLE organizations
    ADD COLUMN IF NOT EXISTS chat_url TEXT,
    ADD COLUMN IF NOT EXISTS meeting_url TEXT,
    ADD COLUMN IF NOT EXISTS drive_url TEXT,
    ADD COLUMN IF NOT EXISTS ai_agent_url TEXT,
    ADD COLUMN IF NOT EXISTS allow_team_override BOOLEAN NOT NULL DEFAULT TRUE;

ALTER TABLE teams
    ADD COLUMN IF NOT EXISTS chat_url TEXT,
    ADD COLUMN IF NOT EXISTS meeting_url TEXT,
    ADD COLUMN IF NOT EXISTS drive_url TEXT,
    ADD COLUMN IF NOT EXISTS ai_agent_url TEXT;
";

/// SQL schema migration 005: Sandbox idle tracking for automatic container shutdown.
pub const SANDBOX_IDLE_TRACKING_SQL: &str = r"
-- 005: Sandbox idle-tracking, for automatic stop after a period of inactivity. plan.md is
-- explicit that container lifecycle should be invisible to the user: start on first use (already
-- true -- see ProjectManagerService::exec_in_sandbox), stop automatically after they've stepped
-- away, rather than a manual Start/Stop toggle or an indefinitely-running container.
ALTER TABLE project_sandboxes
    ADD COLUMN IF NOT EXISTS last_activity_at TIMESTAMPTZ;
";

/// SQL schema migration 006: Personal access tokens, SSH public keys, GPG public keys, and vigilant mode.
pub const PAT_SSH_GPG_KEYS_SQL: &str = r#"
-- 006: Personal Access Tokens (auth for the self-hosted git/apich-vcs remote servers and the
-- `apich` CLI's own network operations), SSH public key storage (compatibility/identity only --
-- no SSH transport server exists yet, tracked separately), GPG public key registry (signature
-- verification for apich-vcs snapshots), and a per-project "vigilant mode" flag that, when on,
-- flags unsigned snapshots as unverified in the timeline UI rather than showing them as if they
-- were equivalent to signed ones.

CREATE TABLE IF NOT EXISTS personal_access_tokens (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(128) NOT NULL,
    token_hash VARCHAR(128) NOT NULL UNIQUE,
    token_prefix VARCHAR(16) NOT NULL, -- first few chars shown in the UI so a user can tell tokens apart without re-revealing them
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_used_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_pat_user ON personal_access_tokens(user_id);

CREATE TABLE IF NOT EXISTS ssh_public_keys (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(128) NOT NULL,
    key_type VARCHAR(32) NOT NULL,
    public_key TEXT NOT NULL,
    fingerprint VARCHAR(64) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT uq_ssh_key_fingerprint UNIQUE (user_id, fingerprint)
);

CREATE TABLE IF NOT EXISTS gpg_public_keys (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(128) NOT NULL,
    public_key TEXT NOT NULL, -- ASCII-armored
    fingerprint VARCHAR(64) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT uq_gpg_key_fingerprint UNIQUE (user_id, fingerprint)
);

ALTER TABLE projects
    ADD COLUMN IF NOT EXISTS vigilant_mode BOOLEAN NOT NULL DEFAULT false;
"#;

/// SQL schema migration 007: Template library tables for versioned documents and templates.
pub const TEMPLATE_LIBRARY_SQL: &str = r#"
-- 007: Template Library -- publishable, versioned templates (LaTeX/Typst/slides/Kanban/note) a
-- user can browse and apply. Visibility is per-template: 'private' (owner only), 'shared' (only
-- the orgs/teams explicitly listed in template_shares -- not necessarily the publisher's own),
-- or 'public' (every user on this instance). Content is stored directly as JSONB on each version
-- rather than as files on disk -- kind-specific shape (e.g. kanban: {"columns":[...]}, note:
-- {"body":"..."}, latex/typst/slides: {"files":[{"path":...,"content":...}]}) interpreted by
-- application code, not by this schema.

CREATE TABLE IF NOT EXISTS templates (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    kind VARCHAR(32) NOT NULL, -- 'kanban', 'note', 'latex', 'typst', 'slides'
    name VARCHAR(128) NOT NULL,
    slug VARCHAR(64) NOT NULL,
    description TEXT,
    owner_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    visibility VARCHAR(16) NOT NULL DEFAULT 'private', -- 'private', 'shared', 'public'
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT uq_template_owner_slug UNIQUE (owner_user_id, slug)
);
CREATE INDEX IF NOT EXISTS idx_templates_kind ON templates(kind);
CREATE INDEX IF NOT EXISTS idx_templates_owner ON templates(owner_user_id);

-- Explicit share targets for 'shared'-visibility templates. Deliberately not limited to the
-- publisher's own org -- a template can be shared to any org/team the publisher chooses.
CREATE TABLE IF NOT EXISTS template_shares (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    template_id UUID NOT NULL REFERENCES templates(id) ON DELETE CASCADE,
    org_id UUID REFERENCES organizations(id) ON DELETE CASCADE,
    team_id UUID REFERENCES teams(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT chk_template_share_target CHECK (
        (org_id IS NOT NULL AND team_id IS NULL) OR (org_id IS NULL AND team_id IS NOT NULL)
    )
);
CREATE UNIQUE INDEX IF NOT EXISTS uq_template_share_org ON template_shares (template_id, org_id) WHERE org_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS uq_template_share_team ON template_shares (template_id, team_id) WHERE team_id IS NOT NULL;

-- Immutable once published, like a package registry version -- "publish a new version" always
-- inserts a new row rather than mutating an old one.
CREATE TABLE IF NOT EXISTS template_versions (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    template_id UUID NOT NULL REFERENCES templates(id) ON DELETE CASCADE,
    version_label VARCHAR(64) NOT NULL, -- free-form, e.g. "1.0.0" or "v3"
    changelog TEXT,
    content JSONB NOT NULL,
    published_by UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT uq_template_version_label UNIQUE (template_id, version_label)
);
CREATE INDEX IF NOT EXISTS idx_template_versions_template ON template_versions(template_id, created_at DESC);
"#;

/// SQL schema migration 008: Invitation code enhancements (multi-use support and optional email restriction)
pub const INVITATION_CODES_ENHANCEMENT_SQL: &str = r"
-- 008: Multi-use invitation codes with configurable usage limits
ALTER TABLE invitations
    ADD COLUMN IF NOT EXISTS max_uses INT NOT NULL DEFAULT 1,
    ADD COLUMN IF NOT EXISTS used_count INT NOT NULL DEFAULT 0;

ALTER TABLE invitations
    ALTER COLUMN email DROP NOT NULL;
";

/// SQL schema migration 009: Passkey full serialization storage for WebAuthn-rs
pub const PASSKEY_JSON_SUPPORT_SQL: &str = r"
-- 009: Store complete WebAuthn Passkey state as JSON
ALTER TABLE fido2_credentials
    ADD COLUMN IF NOT EXISTS passkey_json TEXT;
";


