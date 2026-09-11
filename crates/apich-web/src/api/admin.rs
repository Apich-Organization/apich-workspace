use crate::auth::generate_session_token;
use crate::auth::RequirePlatformAdmin;
use crate::error::WebResult;
use crate::state::AppState;
use apich_db::CreateInvitationDto;
use apich_db::CreateOAuthClientDto;
use apich_db::Invitation;
use apich_db::OAuthClient;
use apich_db::SystemSettings;
use apich_db::UpdateSystemSettingsDto;
use apich_db::User;
use axum::extract::State;
use axum::routing::get;
use axum::routing::post;
use axum::Json;
use axum::Router;
use chrono::Utc;
use serde::Deserialize;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/settings", get(get_settings).put(update_settings))
        .route("/invitations", post(create_invitation))
        .route(
            "/oauth-clients",
            get(list_oauth_clients).post(create_oauth_client),
        )
        .route("/users", get(list_users))
}

#[derive(Debug, Deserialize)]
pub struct InviteUserRequest {
    pub email: String,
    pub org_id: Option<Uuid>,
    pub team_id: Option<Uuid>,
    pub role: Option<String>,
}

async fn get_settings(
    _admin: RequirePlatformAdmin,
    State(state): State<AppState>,
) -> WebResult<Json<SystemSettings>> {
    let repo = state.db.repository();
    let settings = repo.get_system_settings().await?;
    Ok(Json(settings))
}

async fn update_settings(
    _admin: RequirePlatformAdmin,
    State(state): State<AppState>,
    Json(dto): Json<UpdateSystemSettingsDto>,
) -> WebResult<Json<SystemSettings>> {
    let repo = state.db.repository();
    let updated = repo.update_system_settings(dto).await?;
    Ok(Json(updated))
}

async fn create_invitation(
    RequirePlatformAdmin(admin): RequirePlatformAdmin,
    State(state): State<AppState>,
    Json(payload): Json<InviteUserRequest>,
) -> WebResult<Json<Invitation>> {
    let repo = state.db.repository();
    let settings = repo.get_system_settings().await?;

    let token = generate_session_token();
    let expires_at = Utc::now() + chrono::Duration::days(7);

    let invite = repo
        .create_invitation(
            &token,
            CreateInvitationDto {
                email: payload.email.clone(),
                org_id: payload.org_id,
                team_id: payload.team_id,
                role: payload.role,
                inviter_id: Some(admin.id),
                expires_at,
            },
        )
        .await?;

    // Dispatch email in background or recorded queue
    let _ = state
        .mailer
        .send_invitation(
            &settings,
            &invite.email,
            &invite.token,
            &admin.display_name,
            None,
            &state.base_url,
        )
        .await;

    Ok(Json(invite))
}

async fn list_oauth_clients(
    _admin: RequirePlatformAdmin,
    State(state): State<AppState>,
) -> WebResult<Json<Vec<OAuthClient>>> {
    let repo = state.db.repository();
    let clients = repo.list_oauth_clients().await?;
    Ok(Json(clients))
}

async fn create_oauth_client(
    _admin: RequirePlatformAdmin,
    State(state): State<AppState>,
    Json(dto): Json<CreateOAuthClientDto>,
) -> WebResult<Json<OAuthClient>> {
    let repo = state.db.repository();
    let client = repo.create_oauth_client(dto).await?;
    Ok(Json(client))
}

async fn list_users(
    _admin: RequirePlatformAdmin,
    State(state): State<AppState>,
) -> WebResult<Json<Vec<User>>> {
    let repo = state.db.repository();
    let users = repo.list_users().await?;
    Ok(Json(users))
}
