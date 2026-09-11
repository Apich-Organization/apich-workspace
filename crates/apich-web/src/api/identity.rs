use crate::auth::AuthUser;
use crate::error::WebResult;
use crate::state::AppState;
use apich_db::CreateOrganizationDto;
use apich_db::CreateTeamDto;
use apich_db::OrgMemberWithUser;
use apich_db::Organization;
use apich_db::Team;
use apich_db::TeamMemberWithUser;
use apich_db::TeamTreeNode;
use axum::extract::Path;
use axum::extract::State;
use axum::routing::delete;
use axum::routing::get;
use axum::routing::post;
use axum::Json;
use axum::Router;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        // Organizations
        .route("/orgs", post(create_org).get(list_user_orgs))
        .route("/orgs/:id", get(get_org))
        .route(
            "/orgs/:id/members",
            get(list_org_members).post(add_org_member),
        )
        .route("/orgs/:id/members/:user_id", delete(remove_org_member))
        .route("/orgs/:id/teams", get(list_org_teams))
        .route("/orgs/:id/team-tree", get(get_team_tree))
        // Teams
        .route("/teams", post(create_team))
        .route(
            "/teams/:id/members",
            get(list_team_members).post(add_team_member),
        )
        .route("/teams/:id/members/:user_id", delete(remove_team_member))
}

#[derive(Debug, Deserialize)]
pub struct AddMemberRequest {
    pub user_id: Uuid,
    pub role: Option<String>,
}

async fn create_org(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(dto): Json<CreateOrganizationDto>,
) -> WebResult<Json<Organization>> {
    let org = state
        .identity_service
        .create_organization(user.id, dto)
        .await?;
    Ok(Json(org))
}

async fn list_user_orgs(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> WebResult<Json<Vec<Organization>>> {
    let orgs = state
        .identity_service
        .list_user_organizations(user.id)
        .await?;
    Ok(Json(orgs))
}

async fn get_org(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> WebResult<Json<Organization>> {
    let org = state.identity_service.get_organization(user.id, id).await?;
    Ok(Json(org))
}

async fn list_org_members(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> WebResult<Json<Vec<OrgMemberWithUser>>> {
    let members = state.identity_service.list_org_members(user.id, id).await?;
    Ok(Json(members))
}

async fn add_org_member(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<AddMemberRequest>,
) -> WebResult<Json<serde_json::Value>> {
    let role = payload.role.as_deref().unwrap_or("member");
    state
        .identity_service
        .add_org_member(user.id, id, payload.user_id, role)
        .await?;
    Ok(Json(json!({ "status": "success" })))
}

async fn remove_org_member(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path((id, user_id)): Path<(Uuid, Uuid)>,
) -> WebResult<Json<serde_json::Value>> {
    state
        .identity_service
        .remove_org_member(user.id, id, user_id)
        .await?;
    Ok(Json(json!({ "status": "success" })))
}

async fn list_org_teams(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> WebResult<Json<Vec<Team>>> {
    let teams = state
        .identity_service
        .list_teams_by_org(user.id, id)
        .await?;
    Ok(Json(teams))
}

async fn get_team_tree(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> WebResult<Json<Vec<TeamTreeNode>>> {
    let tree = state
        .identity_service
        .get_team_tree_for_org(user.id, id)
        .await?;
    Ok(Json(tree))
}

async fn create_team(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(dto): Json<CreateTeamDto>,
) -> WebResult<Json<Team>> {
    let team = state.identity_service.create_team(user.id, dto).await?;
    Ok(Json(team))
}

async fn list_team_members(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> WebResult<Json<Vec<TeamMemberWithUser>>> {
    let members = state
        .identity_service
        .list_team_members(user.id, id)
        .await?;
    Ok(Json(members))
}

async fn add_team_member(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<AddMemberRequest>,
) -> WebResult<Json<serde_json::Value>> {
    let role = payload.role.as_deref().unwrap_or("member");
    state
        .identity_service
        .add_team_member(user.id, id, payload.user_id, role)
        .await?;
    Ok(Json(json!({ "status": "success" })))
}

async fn remove_team_member(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Path((id, user_id)): Path<(Uuid, Uuid)>,
) -> WebResult<Json<serde_json::Value>> {
    state
        .identity_service
        .remove_team_member(user.id, id, user_id)
        .await?;
    Ok(Json(json!({ "status": "success" })))
}
