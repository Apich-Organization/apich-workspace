use crate::auth::AuthUser;
use crate::error::WebError;
use crate::error::WebResult;
use crate::services::project_manager::FileDiffResult;
use crate::services::project_manager::SnapshotDetailsView;
use crate::services::KnowledgeSyncService;
use crate::services::SqliteTableService;
use crate::state::AppState;
use apich_db::CreateProjectDto;
use apich_db::IdentityPermissionResolver;
use apich_db::Project;
use apich_db::ProjectMember;
use apich_db::ProjectMemberWithUser;
use apich_db::ProjectSandbox;
use apich_vcs::Snapshot;
use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::routing::get;
use axum::routing::post;
use axum::routing::put;
use axum::Json;
use axum::Router;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/projects", post(create_project).get(list_projects))
        .route("/projects/:id", get(get_project).delete(delete_project))
        .route("/projects/:id/sandbox/start", post(start_sandbox))
        .route("/projects/:id/sandbox/stop", post(stop_sandbox))
        .route("/projects/:id/sandbox/status", get(get_sandbox_status))
        .route("/projects/:id/vcs/snapshot", post(snapshot_project))
        .route("/projects/:id/vcs/timeline", get(get_project_timeline))
        .route(
            "/projects/:id/vcs/snapshots/:snap_id/details",
            get(get_snapshot_details_action),
        )
        .route(
            "/projects/:id/vcs/snapshots/:snap_id/diff",
            get(get_snapshot_diff_action),
        )
        .route(
            "/projects/:id/vcs/snapshots/:snap_id/restore",
            post(restore_snapshot_file_action),
        )
        .route(
            "/projects/:id/vcs/snapshots/:snap_id/revert",
            post(revert_snapshot_action),
        )
        .route("/projects/:id/archive", post(archive_project))
        .route(
            "/projects/:id/members",
            get(list_project_members).post(add_project_member),
        )
        .route(
            "/projects/:id/members/:user_id",
            put(update_project_member).delete(remove_project_member),
        )
        // SQLite Tables & Database Files
        .route("/projects/:id/databases", get(list_project_databases))
        .route("/projects/:id/databases/schema", get(get_database_schema))
        .route("/projects/:id/tables/data", get(get_table_data))
        .route("/projects/:id/sql/execute", post(execute_project_sql))
        // Knowledge Hub (Tasks, Kanban, Wiki, Calendar)
        .route("/projects/:id/knowledge/tasks", get(get_knowledge_tasks))
        .route("/projects/:id/knowledge/kanban", get(get_knowledge_kanban))
        .route(
            "/projects/:id/knowledge/tasks/update",
            post(update_knowledge_task),
        )
        .route("/projects/:id/knowledge/graph", get(get_knowledge_graph))
        .route(
            "/projects/:id/knowledge/calendar",
            get(get_knowledge_calendar),
        )
        // Effective Hub Links
        .route("/projects/:id/hub-links", get(get_project_hub_links))
}

#[derive(Debug, Deserialize)]
pub struct CreateProjectRequest {
    pub org_id: Uuid,
    pub team_id: Option<Uuid>,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub storage_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SnapshotRequest {
    pub message: String,
}

async fn create_project(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<CreateProjectRequest>,
) -> WebResult<Json<Project>> {
    // Check if user has permission to create project in this org/team
    let can_create = if let Some(team_id) = payload.team_id {
        IdentityPermissionResolver::can_manage_team(state.db.pool(), user.id, team_id).await?
    } else {
        IdentityPermissionResolver::can_manage_org(state.db.pool(), user.id, payload.org_id).await?
    };

    if !can_create {
        return Err(WebError::Forbidden(
            "Insufficient privileges to create a project in this organization or team".to_string(),
        ));
    }

    let dto = CreateProjectDto {
        org_id: payload.org_id,
        team_id: payload.team_id,
        owner_id: user.id,
        name: payload.name,
        slug: payload.slug,
        description: payload.description,
        storage_path: payload.storage_path.unwrap_or_default(),
        settings: None,
    };

    let proj = state.project_manager.create_project(dto).await?;
    Ok(Json(proj))
}

async fn list_projects(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> WebResult<Json<Vec<Project>>> {
    let repo = state.db.repository();
    let projs = repo.list_projects_for_user(user.id).await?;
    Ok(Json(projs))
}

async fn get_project(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> WebResult<Json<Project>> {
    let can_access =
        IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, id).await?;
    if !can_access {
        return Err(WebError::Forbidden("Access denied to project".to_string()));
    }

    let repo = state.db.repository();
    let proj = repo
        .get_project_by_id(id)
        .await?
        .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

    Ok(Json(proj))
}

async fn start_sandbox(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> WebResult<Json<ProjectSandbox>> {
    let can_edit =
        IdentityPermissionResolver::can_edit_project(state.db.pool(), user.id, id).await?;
    if !can_edit {
        return Err(WebError::Forbidden(
            "Edit privileges required to launch project sandbox".to_string(),
        ));
    }

    let sandbox = state.project_manager.launch_sandbox(id, user.id).await?;
    Ok(Json(sandbox))
}

async fn stop_sandbox(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> WebResult<Json<ProjectSandbox>> {
    let can_edit =
        IdentityPermissionResolver::can_edit_project(state.db.pool(), user.id, id).await?;
    if !can_edit {
        return Err(WebError::Forbidden(
            "Edit privileges required to stop project sandbox".to_string(),
        ));
    }

    let sandbox = state.project_manager.stop_sandbox(id, user.id).await?;
    Ok(Json(sandbox))
}

async fn get_sandbox_status(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> WebResult<Json<Option<ProjectSandbox>>> {
    let can_access =
        IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, id).await?;
    if !can_access {
        return Err(WebError::Forbidden("Access denied to project".to_string()));
    }

    let repo = state.db.repository();
    let sandbox = repo.get_project_sandbox(id, user.id).await?;
    Ok(Json(sandbox))
}

async fn snapshot_project(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<SnapshotRequest>,
) -> WebResult<Json<Option<Snapshot>>> {
    let can_edit =
        IdentityPermissionResolver::can_edit_project(state.db.pool(), user.id, id).await?;
    if !can_edit {
        return Err(WebError::Forbidden(
            "Edit privileges required to snapshot project".to_string(),
        ));
    }

    let summary = state
        .project_manager
        .snapshot_project(id, user.id, &payload.message)
        .await?;

    Ok(Json(summary))
}

async fn get_project_timeline(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> WebResult<Json<Vec<Snapshot>>> {
    let can_access =
        IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, id).await?;
    if !can_access {
        return Err(WebError::Forbidden("Access denied to project".to_string()));
    }

    let timeline = state.project_manager.get_project_timeline(id).await?;
    Ok(Json(timeline))
}

async fn archive_project(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> WebResult<Json<serde_json::Value>> {
    let can_manage =
        IdentityPermissionResolver::can_manage_project(state.db.pool(), user.id, id).await?;
    if !can_manage {
        return Err(WebError::Forbidden(
            "Management privileges required to archive project".to_string(),
        ));
    }

    state.project_manager.archive_project(id).await?;
    Ok(Json(
        json!({ "status": "success", "message": "Project archived" }),
    ))
}

async fn delete_project(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> WebResult<Json<serde_json::Value>> {
    let can_manage =
        IdentityPermissionResolver::can_manage_project(state.db.pool(), user.id, id).await?;
    if !can_manage {
        return Err(WebError::Forbidden(
            "Management privileges required to delete project".to_string(),
        ));
    }

    state.project_manager.delete_project(id).await?;
    Ok(Json(
        json!({ "status": "success", "message": "Project deleted" }),
    ))
}

#[derive(Debug, Deserialize)]
pub struct AddProjectMemberRequest {
    pub user_id: Uuid,
    pub role: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProjectMemberRequest {
    pub role: String,
}

async fn list_project_members(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> WebResult<Json<Vec<ProjectMemberWithUser>>> {
    let can_access =
        IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, id).await?;
    if !can_access {
        return Err(WebError::Forbidden(
            "Access denied to project members".to_string(),
        ));
    }

    let repo = state.db.repository();
    let members = repo.list_project_members(id).await?;
    Ok(Json(members))
}

async fn add_project_member(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<AddProjectMemberRequest>,
) -> WebResult<Json<ProjectMember>> {
    let can_manage =
        IdentityPermissionResolver::can_manage_project(state.db.pool(), user.id, id).await?;
    if !can_manage {
        return Err(WebError::Forbidden(
            "Management privileges required to share project".to_string(),
        ));
    }

    let role = payload.role.as_deref().unwrap_or("editor");
    let repo = state.db.repository();
    let member = repo.add_project_member(id, payload.user_id, role).await?;
    Ok(Json(member))
}

async fn update_project_member(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path((id, target_user_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<UpdateProjectMemberRequest>,
) -> WebResult<Json<ProjectMember>> {
    let can_manage =
        IdentityPermissionResolver::can_manage_project(state.db.pool(), user.id, id).await?;
    if !can_manage {
        return Err(WebError::Forbidden(
            "Management privileges required to update collaborator role".to_string(),
        ));
    }

    let repo = state.db.repository();
    let member = repo
        .update_project_member_role(id, target_user_id, &payload.role)
        .await?;
    Ok(Json(member))
}

async fn remove_project_member(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path((id, target_user_id)): Path<(Uuid, Uuid)>,
) -> WebResult<Json<serde_json::Value>> {
    let can_manage =
        IdentityPermissionResolver::can_manage_project(state.db.pool(), user.id, id).await?;
    if !can_manage {
        return Err(WebError::Forbidden(
            "Management privileges required to remove collaborators".to_string(),
        ));
    }

    let repo = state.db.repository();
    repo.remove_project_member(id, target_user_id).await?;
    Ok(Json(
        json!({ "status": "success", "message": "Collaborator removed from project" }),
    ))
}

async fn list_project_databases(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> WebResult<Json<Vec<crate::services::sqlite_table::DatabaseFileInfo>>> {
    let can_access =
        IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, id).await?;
    if !can_access {
        return Err(WebError::Forbidden("Access denied to project".to_string()));
    }
    let repo = state.db.repository();
    let proj = repo
        .get_project_by_id(id)
        .await?
        .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

    let dbs = SqliteTableService::discover_databases(&proj.storage_path);
    Ok(Json(dbs))
}

#[derive(Debug, Deserialize)]
pub struct DatabaseFileQuery {
    pub file: String,
}

async fn get_database_schema(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(query): Query<DatabaseFileQuery>,
) -> WebResult<Json<crate::services::sqlite_table::DatabaseSchema>> {
    let can_access =
        IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, id).await?;
    if !can_access {
        return Err(WebError::Forbidden("Access denied to project".to_string()));
    }
    let repo = state.db.repository();
    let proj = repo
        .get_project_by_id(id)
        .await?
        .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

    let full_path = SqliteTableService::resolve_db_path(&proj.storage_path, &query.file)?;
    let schema = SqliteTableService::get_database_schema(&full_path)?;
    Ok(Json(schema))
}

#[derive(Debug, Deserialize)]
pub struct TableDataQuery {
    pub file: String,
    pub table: String,
    pub page: Option<usize>,
    pub page_size: Option<usize>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
    pub search: Option<String>,
}

async fn get_table_data(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(query): Query<TableDataQuery>,
) -> WebResult<Json<crate::services::sqlite_table::TableDataPage>> {
    let can_access =
        IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, id).await?;
    if !can_access {
        return Err(WebError::Forbidden("Access denied to project".to_string()));
    }
    let repo = state.db.repository();
    let proj = repo
        .get_project_by_id(id)
        .await?
        .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

    let full_path = SqliteTableService::resolve_db_path(&proj.storage_path, &query.file)?;
    let data = SqliteTableService::get_table_data(
        &full_path,
        &query.table,
        query.page.unwrap_or(1),
        query.page_size.unwrap_or(50),
        query.sort_by.as_deref(),
        query.sort_order.as_deref(),
        query.search.as_deref(),
    )?;
    Ok(Json(data))
}

#[derive(Debug, Deserialize)]
pub struct ExecuteSqlRequest {
    pub file: String,
    pub sql: String,
}

async fn execute_project_sql(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<ExecuteSqlRequest>,
) -> WebResult<Json<crate::services::sqlite_table::SqlExecutionResult>> {
    let can_manage =
        IdentityPermissionResolver::can_manage_project(state.db.pool(), user.id, id).await?;
    if !can_manage {
        return Err(WebError::Forbidden(
            "Management privileges required to execute SQL".to_string(),
        ));
    }
    let repo = state.db.repository();
    let proj = repo
        .get_project_by_id(id)
        .await?
        .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

    let full_path = SqliteTableService::resolve_db_path(&proj.storage_path, &payload.file)?;
    let result = SqliteTableService::execute_sql(&full_path, &payload.sql, 500)?;
    Ok(Json(result))
}

async fn get_knowledge_tasks(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> WebResult<Json<Vec<crate::services::knowledge_sync::MarkdownTask>>> {
    let can_access =
        IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, id).await?;
    if !can_access {
        return Err(WebError::Forbidden("Access denied to project".to_string()));
    }
    let repo = state.db.repository();
    let proj = repo
        .get_project_by_id(id)
        .await?
        .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

    let tasks = KnowledgeSyncService::extract_all_tasks(&proj.storage_path)?;
    Ok(Json(tasks))
}

async fn get_knowledge_kanban(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> WebResult<Json<crate::services::knowledge_sync::KanbanBoard>> {
    let can_access =
        IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, id).await?;
    if !can_access {
        return Err(WebError::Forbidden("Access denied to project".to_string()));
    }
    let repo = state.db.repository();
    let proj = repo
        .get_project_by_id(id)
        .await?
        .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

    let columns = KnowledgeSyncService::parse_kanban_columns(
        &proj.settings,
        KnowledgeSyncService::default_kanban_columns("To Do", "In Progress", "Completed"),
    );
    let board = KnowledgeSyncService::build_kanban_board(&proj.storage_path, &columns, "Unsorted")?;
    Ok(Json(board))
}

#[derive(Debug, Deserialize)]
pub struct UpdateTaskRequest {
    pub file: String,
    pub line_number: usize,
    pub status: String,
}

async fn update_knowledge_task(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateTaskRequest>,
) -> WebResult<Json<serde_json::Value>> {
    let can_access =
        IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, id).await?;
    if !can_access {
        return Err(WebError::Forbidden("Access denied to project".to_string()));
    }
    let repo = state.db.repository();
    let proj = repo
        .get_project_by_id(id)
        .await?
        .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

    // This API's request body predates custom Kanban columns and has no `is_done` field of its
    // own -- resolve it from the project's own column config instead, falling back to the
    // pre-custom-columns "only the literal 'done' id counts as done" rule for either a built-in
    // status id or one that doesn't match any configured column.
    let columns = KnowledgeSyncService::parse_kanban_columns(
        &proj.settings,
        KnowledgeSyncService::default_kanban_columns("To Do", "In Progress", "Completed"),
    );
    let is_done_column = columns
        .iter()
        .find(|c| c.id == payload.status)
        .map_or(payload.status == "done", |c| c.is_done);
    KnowledgeSyncService::update_task_status(
        &proj.storage_path,
        &payload.file,
        payload.line_number,
        &payload.status,
        is_done_column,
    )?;

    Ok(Json(
        json!({ "status": "success", "message": "Task status updated in document" }),
    ))
}

async fn get_knowledge_graph(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> WebResult<Json<crate::services::knowledge_sync::KnowledgeGraph>> {
    let can_access =
        IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, id).await?;
    if !can_access {
        return Err(WebError::Forbidden("Access denied to project".to_string()));
    }
    let repo = state.db.repository();
    let proj = repo
        .get_project_by_id(id)
        .await?
        .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

    let graph = KnowledgeSyncService::build_knowledge_graph(&proj.storage_path)?;
    Ok(Json(graph))
}

async fn get_knowledge_calendar(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> WebResult<Json<Vec<crate::services::knowledge_sync::CalendarEvent>>> {
    let can_access =
        IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, id).await?;
    if !can_access {
        return Err(WebError::Forbidden("Access denied to project".to_string()));
    }
    let repo = state.db.repository();
    let proj = repo
        .get_project_by_id(id)
        .await?
        .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

    let events = KnowledgeSyncService::extract_calendar_events(&proj.storage_path)?;
    Ok(Json(events))
}

async fn get_project_hub_links(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> WebResult<Json<apich_db::EffectiveHubLinks>> {
    let can_access =
        IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, id).await?;
    if !can_access {
        return Err(WebError::Forbidden("Access denied to project".to_string()));
    }
    let repo = state.db.repository();
    let proj = repo
        .get_project_by_id(id)
        .await?
        .ok_or_else(|| WebError::NotFound("Project not found".to_string()))?;

    let links = repo.resolve_hub_links(proj.org_id, proj.team_id).await?;
    Ok(Json(links))
}

#[derive(Debug, Deserialize)]
pub struct SnapshotDiffQuery {
    pub file: String,
}

#[derive(Debug, Deserialize)]
pub struct RestoreFileRequest {
    pub file: String,
    pub target_file: String,
}

async fn get_snapshot_details_action(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path((id, snap_id)): Path<(Uuid, Uuid)>,
) -> WebResult<Json<SnapshotDetailsView>> {
    let can_access =
        IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, id).await?;
    if !can_access {
        return Err(WebError::Forbidden("Access denied to project".to_string()));
    }

    let details = state.project_manager.get_snapshot_details(id, snap_id).await?;
    Ok(Json(details))
}

async fn get_snapshot_diff_action(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path((id, snap_id)): Path<(Uuid, Uuid)>,
    Query(query): Query<SnapshotDiffQuery>,
) -> WebResult<Json<FileDiffResult>> {
    let can_access =
        IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, id).await?;
    if !can_access {
        return Err(WebError::Forbidden("Access denied to project".to_string()));
    }

    let diff = state
        .project_manager
        .get_file_diff_in_snapshot(id, snap_id, &query.file)
        .await?;
    Ok(Json(diff))
}

async fn restore_snapshot_file_action(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path((id, snap_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<RestoreFileRequest>,
) -> WebResult<Json<serde_json::Value>> {
    let can_edit =
        IdentityPermissionResolver::can_edit_project(state.db.pool(), user.id, id).await?;
    if !can_edit {
        return Err(WebError::Forbidden(
            "Edit privileges required to restore file".to_string(),
        ));
    }

    let msg = state
        .project_manager
        .restore_file_from_snapshot(id, user.id, snap_id, &payload.file, &payload.target_file)
        .await?;

    Ok(Json(json!({ "status": "success", "message": msg })))
}

async fn revert_snapshot_action(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path((id, snap_id)): Path<(Uuid, Uuid)>,
) -> WebResult<Json<serde_json::Value>> {
    let can_edit =
        IdentityPermissionResolver::can_edit_project(state.db.pool(), user.id, id).await?;
    if !can_edit {
        return Err(WebError::Forbidden(
            "Edit privileges required to revert project".to_string(),
        ));
    }

    let msg = state
        .project_manager
        .revert_project_to_snapshot(id, user.id, snap_id)
        .await?;

    Ok(Json(json!({ "status": "success", "message": msg })))
}

