use crate::error::Result;
use crate::models::Document;
use crate::models::Workspace;
use sqlx::PgPool;
use sqlx::Postgres;
use sqlx::Transaction;
use uuid::Uuid;

/// Scoped transactional session enforcing PostgreSQL 18 Row-Level Security (RLS).
///
/// In multi-tenant environments, queries executed within this session automatically
/// obey RLS isolation policies based on `app.current_user_id`.
pub struct DbSession<'a> {
    tx: Transaction<'a, Postgres>,
    user_id: Option<Uuid>,
}

impl<'a> DbSession<'a> {
    /// Begin a new transaction scoped to a specific user context for RLS
    ///
    /// # Errors
    /// Returns an error if initiating the transaction or setting session variables fails.
    pub async fn begin(
        pool: &PgPool,
        user_id: Option<Uuid>,
    ) -> Result<Self> {
        let mut tx = pool.begin().await?;
        if let Some(uid) = user_id {
            sqlx::query(&format!("SET LOCAL app.current_user_id = '{uid}';"))
                .execute(&mut *tx)
                .await?;
        } else {
            sqlx::query("SET LOCAL app.current_user_id = '';")
                .execute(&mut *tx)
                .await?;
        }
        Ok(Self { tx, user_id })
    }

    /// Begin a transaction that bypasses RLS (for background tasks or system administration)
    ///
    /// # Errors
    /// Returns an error if initiating the transaction or setting the bypass RLS flag fails.
    pub async fn begin_system(pool: &PgPool) -> Result<Self> {
        let mut tx = pool.begin().await?;
        sqlx::query("SET LOCAL app.bypass_rls = 'on';")
            .execute(&mut *tx)
            .await?;
        Ok(Self { tx, user_id: None })
    }

    /// Get current session user ID
    #[must_use]
    pub const fn user_id(&self) -> Option<Uuid> {
        self.user_id
    }

    /// Mutable access to the inner transaction
    pub const fn tx_mut(&mut self) -> &mut Transaction<'a, Postgres> {
        &mut self.tx
    }

    /// Commit the transaction
    ///
    /// # Errors
    /// Returns an error if committing the database transaction fails.
    pub async fn commit(self) -> Result<()> {
        self.tx.commit().await?;
        Ok(())
    }

    /// Rollback the transaction
    ///
    /// # Errors
    /// Returns an error if rolling back the database transaction fails.
    pub async fn rollback(self) -> Result<()> {
        self.tx.rollback().await?;
        Ok(())
    }

    /// Query all workspaces accessible by the current session under RLS
    ///
    /// # Errors
    /// Returns an error if querying workspaces from the database fails.
    pub async fn list_visible_workspaces(&mut self) -> Result<Vec<Workspace>> {
        let rows = sqlx::query_as::<_, Workspace>("SELECT * FROM workspaces ORDER BY name ASC")
            .fetch_all(&mut *self.tx)
            .await?;
        Ok(rows)
    }

    /// Query documents accessible by the current session under RLS
    ///
    /// # Errors
    /// Returns an error if querying documents from the database fails.
    pub async fn list_visible_documents(
        &mut self,
        workspace_id: Uuid,
    ) -> Result<Vec<Document>> {
        let rows = sqlx::query_as::<_, Document>(
            "SELECT * FROM documents WHERE workspace_id = $1 ORDER BY updated_at DESC",
        )
        .bind(workspace_id)
        .fetch_all(&mut *self.tx)
        .await?;
        Ok(rows)
    }

    /// Query a document by path, protected by RLS
    ///
    /// # Errors
    /// Returns an error if querying the document from the database fails.
    pub async fn get_visible_document_by_path(
        &mut self,
        workspace_id: Uuid,
        rel_path: &str,
    ) -> Result<Option<Document>> {
        let doc = sqlx::query_as::<_, Document>(
            "SELECT * FROM documents WHERE workspace_id = $1 AND rel_path = $2",
        )
        .bind(workspace_id)
        .bind(rel_path)
        .fetch_optional(&mut *self.tx)
        .await?;
        Ok(doc)
    }
}
