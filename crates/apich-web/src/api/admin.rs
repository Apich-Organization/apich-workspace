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
use axum::Json;
use axum::Router;
use chrono::Utc;
use serde::Deserialize;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/settings", get(get_settings).put(update_settings))
        .route("/invitations", get(list_invitations).post(create_invitation))
        .route("/invitations/:id", axum::routing::delete(delete_invitation))
        .route(
            "/oauth-clients",
            get(list_oauth_clients).post(create_oauth_client),
        )
        .route("/users", get(list_users))
}

#[derive(Debug, Deserialize)]
pub struct InviteUserRequest {
    pub code: Option<String>,
    pub email: Option<String>,
    pub org_id: Option<Uuid>,
    pub team_id: Option<Uuid>,
    pub role: Option<String>,
    pub max_uses: Option<i32>,
    pub expires_in_days: Option<i64>,
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

    let token = payload
        .code
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(generate_session_token);
    let days = payload.expires_in_days.unwrap_or(7).clamp(1, 365);
    let expires_at = Utc::now()
        .checked_add_signed(chrono::Duration::days(days))
        .unwrap_or_else(Utc::now);
    let max_uses = payload.max_uses.unwrap_or(1).max(1);

    let invite = repo
        .create_invitation(
            &token,
            CreateInvitationDto {
                token: Some(token.clone()),
                email: payload.email.clone(),
                org_id: payload.org_id,
                team_id: payload.team_id,
                role: payload.role,
                inviter_id: Some(admin.id),
                max_uses: Some(max_uses),
                expires_at,
            },
        )
        .await?;

    // Dispatch email if recipient email was provided
    if let Some(ref target_email) = invite.email {
        if !target_email.trim().is_empty() {
            let _ = state
                .mailer
                .send_invitation(
                    &settings,
                    target_email,
                    &invite.token,
                    &admin.display_name,
                    None,
                    &state.base_url,
                )
                .await;
        }
    }

    Ok(Json(invite))
}

async fn list_invitations(
    _admin: RequirePlatformAdmin,
    State(state): State<AppState>,
) -> WebResult<Json<Vec<Invitation>>> {
    let repo = state.db.repository();
    let list = repo.list_invitations().await?;
    Ok(Json(list))
}

async fn delete_invitation(
    _admin: RequirePlatformAdmin,
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> WebResult<Json<bool>> {
    let repo = state.db.repository();
    repo.delete_invitation(id).await?;
    Ok(Json(true))
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
