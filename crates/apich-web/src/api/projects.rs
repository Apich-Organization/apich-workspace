use crate::{
    auth::AuthUser,
    error::{WebError, WebResult},
    state::AppState,
};
use apich_db::{
    CreateProjectDto, IdentityPermissionResolver, Project, ProjectMember, ProjectMemberWithUser,
    ProjectSandbox,
};
use apich_vcs::Snapshot;
use axum::{
    extract::{Path, State},
    routing::{get, post, put},
    Json, Router,
};
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
        .route("/projects/:id/archive", post(archive_project))
        .route(
            "/projects/:id/members",
            get(list_project_members).post(add_project_member),
        )
        .route(
            "/projects/:id/members/:user_id",
            put(update_project_member).delete(remove_project_member),
        )
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
    let can_access = IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, id).await?;
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
    let can_edit = IdentityPermissionResolver::can_edit_project(state.db.pool(), user.id, id).await?;
    if !can_edit {
        return Err(WebError::Forbidden("Edit privileges required to launch project sandbox".to_string()));
    }

    let sandbox = state.project_manager.launch_sandbox(id, user.id).await?;
    Ok(Json(sandbox))
}

async fn stop_sandbox(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> WebResult<Json<ProjectSandbox>> {
    let can_edit = IdentityPermissionResolver::can_edit_project(state.db.pool(), user.id, id).await?;
    if !can_edit {
        return Err(WebError::Forbidden("Edit privileges required to stop project sandbox".to_string()));
    }

    let sandbox = state.project_manager.stop_sandbox(id, user.id).await?;
    Ok(Json(sandbox))
}

async fn get_sandbox_status(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> WebResult<Json<Option<ProjectSandbox>>> {
    let can_access = IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, id).await?;
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
    let can_edit = IdentityPermissionResolver::can_edit_project(state.db.pool(), user.id, id).await?;
    if !can_edit {
        return Err(WebError::Forbidden("Edit privileges required to snapshot project".to_string()));
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
    let can_access = IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, id).await?;
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
    let can_manage = IdentityPermissionResolver::can_manage_project(state.db.pool(), user.id, id).await?;
    if !can_manage {
        return Err(WebError::Forbidden("Management privileges required to archive project".to_string()));
    }

    state.project_manager.archive_project(id).await?;
    Ok(Json(json!({ "status": "success", "message": "Project archived" })))
}

async fn delete_project(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> WebResult<Json<serde_json::Value>> {
    let can_manage = IdentityPermissionResolver::can_manage_project(state.db.pool(), user.id, id).await?;
    if !can_manage {
        return Err(WebError::Forbidden("Management privileges required to delete project".to_string()));
    }

    state.project_manager.delete_project(id).await?;
    Ok(Json(json!({ "status": "success", "message": "Project deleted" })))
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
    let can_access = IdentityPermissionResolver::can_access_project(state.db.pool(), user.id, id).await?;
    if !can_access {
        return Err(WebError::Forbidden("Access denied to project members".to_string()));
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
    let can_manage = IdentityPermissionResolver::can_manage_project(state.db.pool(), user.id, id).await?;
    if !can_manage {
        return Err(WebError::Forbidden("Management privileges required to share project".to_string()));
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
    let can_manage = IdentityPermissionResolver::can_manage_project(state.db.pool(), user.id, id).await?;
    if !can_manage {
        return Err(WebError::Forbidden("Management privileges required to update collaborator role".to_string()));
    }

    let repo = state.db.repository();
    let member = repo.update_project_member_role(id, target_user_id, &payload.role).await?;
    Ok(Json(member))
}

async fn remove_project_member(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path((id, target_user_id)): Path<(Uuid, Uuid)>,
) -> WebResult<Json<serde_json::Value>> {
    let can_manage = IdentityPermissionResolver::can_manage_project(state.db.pool(), user.id, id).await?;
    if !can_manage {
        return Err(WebError::Forbidden("Management privileges required to remove collaborators".to_string()));
    }

    let repo = state.db.repository();
    repo.remove_project_member(id, target_user_id).await?;
    Ok(Json(json!({ "status": "success", "message": "Collaborator removed from project" })))
}
