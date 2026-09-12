use crate::config::PostgresConfig;
use crate::error::DbError;
use crate::error::Result;
use sqlx::PgPool;
use sqlx::Row;
use tracing::info;

/// Manages PostgreSQL security roles, authentication methods, and database privileges
pub struct PermissionManager;

impl PermissionManager {
    /// Initialize application user and enforce the principle of least privilege
    ///
    /// # Errors
    /// Returns an error if querying or executing database permission queries fails.
    pub async fn setup_least_privilege(
        admin_pool: &PgPool,
        config: &PostgresConfig,
    ) -> Result<()> {
        let app_user = &config.app_user;
        let app_password = &config.app_password;
        let database = &config.database;

        info!(
            app_user = %app_user,
            database = %database,
            "Configuring PostgreSQL least-privilege security roles"
        );

        // 1. Create application user role if it doesn't already exist
        let role_exists_query = "SELECT 1 FROM pg_roles WHERE rolname = $1";
        let exists: Option<i32> = sqlx::query_scalar(role_exists_query)
            .bind(app_user)
            .fetch_optional(admin_pool)
            .await?;

        if exists.is_none() {
            // Note: Postgres identifiers & passwords in DDL statements cannot use parameterized bindings directly
            let create_role_sql = format!(
                "CREATE ROLE \"{}\" WITH LOGIN PASSWORD '{}' NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS",
                app_user,
                app_password.replace('\'', "''")
            );
            sqlx::query(&create_role_sql).execute(admin_pool).await?;
            info!(role = %app_user, "Created application user role");
        } else {
            let update_pwd_sql = format!(
                "ALTER ROLE \"{}\" WITH LOGIN PASSWORD '{}' NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS",
                app_user,
                app_password.replace('\'', "''")
            );
            sqlx::query(&update_pwd_sql).execute(admin_pool).await?;
            info!(role = %app_user, "Updated application user credentials and role attributes");
        }

        // 2. Grant CONNECT on database
        let grant_connect_sql =
            format!("GRANT CONNECT ON DATABASE \"{database}\" TO \"{app_user}\"");
        sqlx::query(&grant_connect_sql).execute(admin_pool).await?;

        // 3. Grant schema usage and creation
        let grant_schema_sql = format!("GRANT USAGE, CREATE ON SCHEMA public TO \"{app_user}\"");
        sqlx::query(&grant_schema_sql).execute(admin_pool).await?;

        // 4. Grant table CRUD permissions
        let grant_tables_sql = format!(
            "GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO \"{app_user}\""
        );
        sqlx::query(&grant_tables_sql).execute(admin_pool).await?;

        // 5. Grant sequence permissions
        let grant_sequences_sql =
            format!("GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO \"{app_user}\"");
        sqlx::query(&grant_sequences_sql)
            .execute(admin_pool)
            .await?;

        // 6. Set default privileges for newly created tables & sequences in schema public
        let default_tables_sql = format!(
            "ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO \"{app_user}\""
        );
        sqlx::query(&default_tables_sql).execute(admin_pool).await?;

        let default_sequences_sql = format!(
            "ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT USAGE, SELECT ON SEQUENCES TO \"{app_user}\""
        );
        sqlx::query(&default_sequences_sql)
            .execute(admin_pool)
            .await?;

        info!(
            app_user = %app_user,
            "Application role least-privilege permissions applied successfully"
        );
        Ok(())
    }

    /// Verify that application user has proper access but lacks superuser powers
    ///
    /// # Errors
    /// Returns an error if querying roles fails or if the application user has superuser privileges.
    pub async fn verify_permissions(
        app_pool: &PgPool,
        app_user: &str,
    ) -> Result<bool> {
        // 1. Verify user can query current_user
        let row = sqlx::query("SELECT current_user AS username, rolsuper, rolcreatedb FROM pg_roles WHERE rolname = $1")
            .bind(app_user)
            .fetch_one(app_pool)
            .await?;

        let username: String = row.try_get("username")?;
        let is_superuser: bool = row.try_get("rolsuper")?;
        let can_create_db: bool = row.try_get("rolcreatedb")?;

        if is_superuser || can_create_db {
            return Err(DbError::PermissionError(format!(
                "Security violation: Application user '{username}' has superuser or createdb privilege!"
            )));
        }

        info!(
            user = %username,
            "Verified app user has least-privilege security attributes (non-superuser)"
        );
        Ok(true)
    }
}

/// Evaluates hierarchical identity tree permissions across Organizations, recursive Teams,
/// and Projects.
///
/// Hierarchy rule:
/// 1. Platform Admin has omnipotent access.
/// 2. Organization Owner/Admin can manage everything in their organization (all teams, subteams, projects, members).
/// 3. Team Admin can manage their team, all nested descendant sub-teams, projects under that subtree, and team members.
/// 4. Project Owner can manage their project and its sandboxes.
pub struct IdentityPermissionResolver;

impl IdentityPermissionResolver {
    /// Check if user is a platform-wide administrator
    ///
    /// # Errors
    /// Returns an error if querying database user records fails.
    pub async fn is_platform_admin(
        pool: &PgPool,
        user_id: uuid::Uuid,
    ) -> Result<bool> {
        let is_admin = sqlx::query_scalar::<_, bool>(
            "SELECT (is_platform_admin OR role = 'admin') FROM users WHERE id = $1 AND is_active = true",
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await?
        .unwrap_or(false);

        Ok(is_admin)
    }

    /// Check if user can manage an Organization
    ///
    /// # Errors
    /// Returns an error if querying database organization member records fails.
    pub async fn can_manage_org(
        pool: &PgPool,
        user_id: uuid::Uuid,
        org_id: uuid::Uuid,
    ) -> Result<bool> {
        if Self::is_platform_admin(pool, user_id).await? {
            return Ok(true);
        }

        let is_org_admin = sqlx::query_scalar::<_, bool>(
            r"
            SELECT EXISTS (
                SELECT 1 FROM org_members
                WHERE org_id = $1 AND user_id = $2 AND role IN ('owner', 'admin')
            )
            ",
        )
        .bind(org_id)
        .bind(user_id)
        .fetch_one(pool)
        .await?;

        Ok(is_org_admin)
    }

    /// Check if user can manage a Team (including inherited rights from Org Admin or parent Team Admin)
    ///
    /// # Errors
    /// Returns an error if querying team, organization, or ancestor team hierarchies fails.
    pub async fn can_manage_team(
        pool: &PgPool,
        user_id: uuid::Uuid,
        team_id: uuid::Uuid,
    ) -> Result<bool> {
        if Self::is_platform_admin(pool, user_id).await? {
            return Ok(true);
        }

        // 1. Get the org_id of the team
        let org_id: Option<uuid::Uuid> =
            sqlx::query_scalar("SELECT org_id FROM teams WHERE id = $1")
                .bind(team_id)
                .fetch_optional(pool)
                .await?;

        let Some(org_id) = org_id else {
            return Ok(false);
        };

        // 2. Org owner / admin has management rights over all teams in that org
        let is_org_admin = sqlx::query_scalar::<_, bool>(
            r"
            SELECT EXISTS (
                SELECT 1 FROM org_members
                WHERE org_id = $1 AND user_id = $2 AND role IN ('owner', 'admin')
            )
            ",
        )
        .bind(org_id)
        .bind(user_id)
        .fetch_one(pool)
        .await?;

        if is_org_admin {
            return Ok(true);
        }

        // 3. Check if user is an admin of this team OR any ancestor team up the hierarchy tree
        let is_ancestor_admin = sqlx::query_scalar::<_, bool>(
            r"
            WITH RECURSIVE team_ancestors AS (
                SELECT id, parent_team_id FROM teams WHERE id = $1
                UNION ALL
                SELECT t.id, t.parent_team_id
                FROM teams t
                JOIN team_ancestors a ON t.id = a.parent_team_id
            )
            SELECT EXISTS (
                SELECT 1 FROM team_members tm
                JOIN team_ancestors ta ON tm.team_id = ta.id
                WHERE tm.user_id = $2 AND tm.role IN ('owner', 'admin')
            )
            ",
        )
        .bind(team_id)
        .bind(user_id)
        .fetch_one(pool)
        .await?;

        Ok(is_ancestor_admin)
    }

    /// Check if user can manage a Project (e.g. edit, archive, delete, config)
    ///
    /// # Errors
    /// Returns an error if querying project, organization, or team membership fails.
    pub async fn can_manage_project(
        pool: &PgPool,
        user_id: uuid::Uuid,
        project_id: uuid::Uuid,
    ) -> Result<bool> {
        if Self::is_platform_admin(pool, user_id).await? {
            return Ok(true);
        }

        // Direct project membership with owner or admin role
        let is_proj_admin = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (SELECT 1 FROM project_members WHERE project_id = $1 AND user_id = $2 AND role IN ('owner', 'admin'))",
        )
        .bind(project_id)
        .bind(user_id)
        .fetch_one(pool)
        .await?;

        if is_proj_admin {
            return Ok(true);
        }

        // Fetch project owner, org_id, team_id
        let row = sqlx::query(
            "SELECT owner_id, org_id, team_id FROM projects WHERE id = $1 AND status != 'deleted'",
        )
        .bind(project_id)
        .fetch_optional(pool)
        .await?;

        let Some(row) = row else {
            return Ok(false);
        };

        let owner_id: uuid::Uuid = row.try_get("owner_id")?;
        if owner_id == user_id {
            return Ok(true);
        }

        let org_id: uuid::Uuid = row.try_get("org_id")?;
        if Self::can_manage_org(pool, user_id, org_id).await? {
            return Ok(true);
        }

        let team_id: Option<uuid::Uuid> = row.try_get("team_id")?;
        if let Some(tid) = team_id {
            if Self::can_manage_team(pool, user_id, tid).await? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Check if user can edit / write / snapshot a Project
    ///
    /// # Errors
    /// Returns an error if querying project permissions or team membership fails.
    pub async fn can_edit_project(
        pool: &PgPool,
        user_id: uuid::Uuid,
        project_id: uuid::Uuid,
    ) -> Result<bool> {
        if Self::can_manage_project(pool, user_id, project_id).await? {
            return Ok(true);
        }

        // Direct project membership with editor role
        let is_proj_editor = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (SELECT 1 FROM project_members WHERE project_id = $1 AND user_id = $2 AND role = 'editor')",
        )
        .bind(project_id)
        .bind(user_id)
        .fetch_one(pool)
        .await?;

        if is_proj_editor {
            return Ok(true);
        }

        // Check if user is member of project's team
        let row = sqlx::query("SELECT team_id FROM projects WHERE id = $1 AND status != 'deleted'")
            .bind(project_id)
            .fetch_optional(pool)
            .await?;

        if let Some(r) = row {
            let team_id: Option<uuid::Uuid> = r.try_get("team_id")?;
            if let Some(tid) = team_id {
                let is_team_member = sqlx::query_scalar::<_, bool>(
                    "SELECT EXISTS (SELECT 1 FROM team_members WHERE team_id = $1 AND user_id = $2)",
                )
                .bind(tid)
                .bind(user_id)
                .fetch_one(pool)
                .await?;

                if is_team_member {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Check if user can access / view a Project
    ///
    /// # Errors
    /// Returns an error if querying project permissions or organization membership fails.
    pub async fn can_access_project(
        pool: &PgPool,
        user_id: uuid::Uuid,
        project_id: uuid::Uuid,
    ) -> Result<bool> {
        if Self::can_edit_project(pool, user_id, project_id).await? {
            return Ok(true);
        }

        // Direct project membership with viewer role
        let is_proj_member = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (SELECT 1 FROM project_members WHERE project_id = $1 AND user_id = $2)",
        )
        .bind(project_id)
        .bind(user_id)
        .fetch_one(pool)
        .await?;

        if is_proj_member {
            return Ok(true);
        }

        let row = sqlx::query("SELECT org_id FROM projects WHERE id = $1 AND status != 'deleted'")
            .bind(project_id)
            .fetch_optional(pool)
            .await?;

        let Some(row) = row else {
            return Ok(false);
        };

        let org_id: uuid::Uuid = row.try_get("org_id")?;

        // If part of org, check org membership
        let is_org_member = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (SELECT 1 FROM org_members WHERE org_id = $1 AND user_id = $2)",
        )
        .bind(org_id)
        .bind(user_id)
        .fetch_one(pool)
        .await?;

        Ok(is_org_member)
    }

    /// Retrieve all subteams recursively descending from a given team node
    ///
    /// # Errors
    /// Returns an error if querying recursive team hierarchies fails.
    pub async fn get_descendant_team_ids(
        pool: &PgPool,
        root_team_id: uuid::Uuid,
    ) -> Result<Vec<uuid::Uuid>> {
        let ids = sqlx::query_scalar::<_, uuid::Uuid>(
            r"
            WITH RECURSIVE team_subtrees AS (
                SELECT id FROM teams WHERE id = $1
                UNION ALL
                SELECT t.id
                FROM teams t
                JOIN team_subtrees ts ON t.parent_team_id = ts.id
            )
            SELECT id FROM team_subtrees
            ",
        )
        .bind(root_team_id)
        .fetch_all(pool)
        .await?;

        Ok(ids)
    }

    /// A template's visibility (see `TemplateVisibility`) is per-template, not derived from
    /// project/org membership like everything else in this file: 'private' means only its owner,
    /// 'public' means every user on the instance, and 'shared' means whatever specific orgs/teams
    /// are listed in `template_shares` -- deliberately not limited to the owner's own org, a
    /// template can be shared to any org/team the owner chooses.
    ///
    /// # Errors
    /// Returns an error if querying template metadata or share records fails.
    pub async fn can_access_template(
        pool: &PgPool,
        user_id: uuid::Uuid,
        template_id: uuid::Uuid,
    ) -> Result<bool> {
        let row = sqlx::query("SELECT owner_user_id, visibility FROM templates WHERE id = $1")
            .bind(template_id)
            .fetch_optional(pool)
            .await?;
        let Some(row) = row else { return Ok(false) };

        let owner_user_id: uuid::Uuid = row.try_get("owner_user_id")?;
        if owner_user_id == user_id {
            return Ok(true);
        }

        let visibility: String = row.try_get("visibility")?;
        match visibility.as_str() {
            | "public" => Ok(true),
            | "shared" => {
                let is_shared = sqlx::query_scalar::<_, bool>(
                    r"
                    SELECT EXISTS (
                        SELECT 1 FROM template_shares ts
                        LEFT JOIN org_members om ON ts.org_id = om.org_id AND om.user_id = $2
                        LEFT JOIN team_members tm ON ts.team_id = tm.team_id AND tm.user_id = $2
                        WHERE ts.template_id = $1 AND (om.user_id IS NOT NULL OR tm.user_id IS NOT NULL)
                    )
                    ",
                )
                .bind(template_id)
                .bind(user_id)
                .fetch_one(pool)
                .await?;
                Ok(is_shared)
            },
            | _ => Ok(false), /* 'private' (or anything unrecognized): owner-only, already checked above */
        }
    }

    /// Only a template's owner can publish new versions, edit its visibility/shares, or delete
    /// it -- unlike projects, a template has no member/admin roles of its own to delegate this to.
    ///
    /// # Errors
    /// Returns an error if querying template owner records fails.
    pub async fn can_manage_template(
        pool: &PgPool,
        user_id: uuid::Uuid,
        template_id: uuid::Uuid,
    ) -> Result<bool> {
        let owner_user_id = sqlx::query_scalar::<_, uuid::Uuid>(
            "SELECT owner_user_id FROM templates WHERE id = $1",
        )
        .bind(template_id)
        .fetch_optional(pool)
        .await?;
        Ok(owner_user_id == Some(user_id))
    }
}
