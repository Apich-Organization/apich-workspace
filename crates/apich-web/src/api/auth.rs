use crate::auth::build_clear_cookie;
use crate::auth::build_session_cookie;
use crate::auth::generate_session_token;
use crate::auth::hash_password;
use crate::auth::hash_session_token;
use crate::auth::verify_password;
use crate::auth::AuthUser;
use crate::error::WebError;
use crate::error::WebResult;
use crate::state::AppState;
use apich_db::CreateUserDto;
use apich_db::User;
use apich_db::UserRole;
use axum::extract::State;
use axum::http::header;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::get;
use axum::routing::post;
use axum::Json;
use axum::Router;
use base64::prelude::*;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/me", get(get_current_user))
        .route("/passkey/register/start", post(passkey_register_start))
        .route("/passkey/register/finish", post(passkey_register_finish))
        .route("/passkey/auth/start", post(passkey_auth_start))
        .route("/passkey/auth/finish", post(passkey_auth_finish))
        .route("/fido2/register/challenge", post(passkey_register_start))
        .route("/fido2/register/finish", post(passkey_register_finish))
        .route("/fido2/auth/challenge", post(passkey_auth_start))
        .route("/fido2/auth/finish", post(passkey_auth_finish))
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
    pub display_name: String,
    pub invite_token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    #[serde(alias = "login")]
    pub username_or_email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub user: User,
    pub token: String,
}

async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> WebResult<Response> {
    let repo = state.db.repository();
    let settings = repo.get_system_settings().await?;

    // Verify registration mode
    match settings.registration_mode.as_str() {
        | "admin_only" => {
            return Err(WebError::Forbidden(
                "Self-serve registration is disabled on this platform".to_string(),
            ));
        },
        | "invite_only" => {
            let token = payload.invite_token.as_deref().ok_or_else(|| {
                WebError::Forbidden("An invitation code is required to register".to_string())
            })?;
            let invite = repo.get_invitation_by_token(token).await?.ok_or_else(|| {
                WebError::BadRequest("Invalid or expired invitation token".to_string())
            })?;

            if invite.email.to_lowercase() != payload.email.to_lowercase() {
                return Err(WebError::BadRequest(
                    "Registration email does not match invitation recipient".to_string(),
                ));
            }

            // Mark invitation used
            repo.mark_invitation_used(token).await?;
        },
        | _ => {}, // "open" mode allowed
    }

    if repo
        .get_user_by_username(&payload.username)
        .await?
        .is_some()
    {
        return Err(WebError::Conflict("Username already taken".to_string()));
    }
    if repo.get_user_by_email(&payload.email).await?.is_some() {
        return Err(WebError::Conflict("Email already registered".to_string()));
    }

    let password_hash = hash_password(&payload.password)?;
    let user = repo
        .create_user(CreateUserDto {
            username: payload.username,
            email: payload.email,
            password_hash,
            display_name: payload.display_name,
            role: Some(UserRole::Member),
            is_platform_admin: Some(false),
            storage_quota_bytes: None,
        })
        .await?;

    // Create initial session
    let token = generate_session_token();
    let token_hash = hash_session_token(&token);
    let expires_at = Utc::now() + chrono::Duration::days(14);
    repo.create_user_session(user.id, &token_hash, expires_at, None, None)
        .await?;

    let cookie = build_session_cookie(&token, 14 * 86400);
    let body = Json(AuthResponse { user, token });

    Ok(([(header::SET_COOKIE, cookie)], body).into_response())
}

async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> WebResult<Response> {
    let repo = state.db.repository();
    let user = if payload.username_or_email.contains('@') {
        repo.get_user_by_email(&payload.username_or_email).await?
    } else {
        repo.get_user_by_username(&payload.username_or_email)
            .await?
    };

    let user = user.ok_or(WebError::InvalidCredentials)?;
    if !user.is_active {
        return Err(WebError::Forbidden(
            "Account has been deactivated".to_string(),
        ));
    }

    let valid = verify_password(&payload.password, &user.password_hash)?;
    if !valid {
        return Err(WebError::InvalidCredentials);
    }

    let token = generate_session_token();
    let token_hash = hash_session_token(&token);
    let expires_at = Utc::now() + chrono::Duration::days(14);
    repo.create_user_session(user.id, &token_hash, expires_at, None, None)
        .await?;

    let cookie = build_session_cookie(&token, 14 * 86400);
    let body = Json(AuthResponse { user, token });

    Ok(([(header::SET_COOKIE, cookie)], body).into_response())
}

async fn logout(
    auth_user: Option<AuthUser>,
    State(state): State<AppState>,
) -> WebResult<Response> {
    if let Some(AuthUser(user)) = auth_user {
        // Clear all active sessions or current session
        let _ = sqlx::query("DELETE FROM user_sessions WHERE user_id = $1")
            .bind(user.id)
            .execute(state.db.pool())
            .await;
    }

    let clear_cookie = build_clear_cookie();
    Ok((
        [(header::SET_COOKIE, clear_cookie)],
        Json(json!({ "message": "Successfully signed out" })),
    )
        .into_response())
}

async fn get_current_user(AuthUser(user): AuthUser) -> WebResult<Json<User>> {
    Ok(Json(user))
}

// --- FIDO2 / WebAuthn Passkey Handlers ---

#[derive(Debug, Serialize)]
pub struct PasskeyRegisterStartResponse {
    pub challenge: String,
    pub rp: PasskeyRp,
    pub user: PasskeyUser,
}

#[derive(Debug, Serialize)]
pub struct PasskeyRp {
    pub name: String,
    pub id: String,
}

#[derive(Debug, Serialize)]
pub struct PasskeyUser {
    pub id: String,
    pub name: String,
    pub display_name: String,
}

#[derive(Debug, Deserialize)]
pub struct PasskeyRegisterFinishRequest {
    pub credential_id: String,
    pub public_key_base64: String,
    pub device_name: Option<String>,
}

async fn passkey_register_start(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> WebResult<Json<PasskeyRegisterStartResponse>> {
    let challenge = state.passkey_manager.generate_challenge(Some(user.id));
    Ok(Json(PasskeyRegisterStartResponse {
        challenge,
        rp: PasskeyRp {
            name: "APICH Workspace".to_string(),
            id: "localhost".to_string(),
        },
        user: PasskeyUser {
            id: user.id.to_string(),
            name: user.username,
            display_name: user.display_name,
        },
    }))
}

async fn passkey_register_finish(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<PasskeyRegisterFinishRequest>,
) -> WebResult<Json<serde_json::Value>> {
    let public_key_bytes = BASE64_STANDARD
        .decode(&payload.public_key_base64)
        .or_else(|_| BASE64_URL_SAFE_NO_PAD.decode(&payload.public_key_base64))
        .map_err(|e| WebError::PasskeyError(format!("Invalid public key base64: {}", e)))?;

    let device_name = payload
        .device_name
        .unwrap_or_else(|| "Security Key".to_string());

    let repo = state.db.repository();
    repo.save_fido2_credential(
        user.id,
        &payload.credential_id,
        &public_key_bytes,
        0,
        &device_name,
        None,
    )
    .await?;

    Ok(Json(json!({
        "status": "success",
        "message": "Passkey successfully enrolled"
    })))
}

#[derive(Debug, Serialize)]
pub struct PasskeyAuthStartResponse {
    pub challenge: String,
    pub timeout: u64,
}

#[derive(Debug, Deserialize)]
pub struct PasskeyAuthFinishRequest {
    pub credential_id: String,
    pub challenge: String,
    pub client_data_json_base64: String,
    pub auth_data_base64: String,
    pub signature_base64: String,
}

async fn passkey_auth_start(
    State(state): State<AppState>
) -> WebResult<Json<PasskeyAuthStartResponse>> {
    let challenge = state.passkey_manager.generate_challenge(None);
    Ok(Json(PasskeyAuthStartResponse {
        challenge,
        timeout: 60000,
    }))
}

async fn passkey_auth_finish(
    State(state): State<AppState>,
    Json(payload): Json<PasskeyAuthFinishRequest>,
) -> WebResult<Response> {
    // 1. Verify challenge was registered
    let _ = state
        .passkey_manager
        .verify_and_consume_challenge(&payload.challenge)?;

    // 2. Fetch credential from DB
    let repo = state.db.repository();
    let cred = repo
        .get_fido2_credential_by_id(&payload.credential_id)
        .await?
        .ok_or_else(|| WebError::PasskeyError("Unknown security credential".to_string()))?;

    // 3. Decode WebAuthn payload
    let client_data_json = BASE64_STANDARD
        .decode(&payload.client_data_json_base64)
        .or_else(|_| BASE64_URL_SAFE_NO_PAD.decode(&payload.client_data_json_base64))
        .map_err(|e| WebError::PasskeyError(format!("Invalid clientDataJSON: {}", e)))?;

    let auth_data = BASE64_STANDARD
        .decode(&payload.auth_data_base64)
        .or_else(|_| BASE64_URL_SAFE_NO_PAD.decode(&payload.auth_data_base64))
        .map_err(|e| WebError::PasskeyError(format!("Invalid authData: {}", e)))?;

    let signature = BASE64_STANDARD
        .decode(&payload.signature_base64)
        .or_else(|_| BASE64_URL_SAFE_NO_PAD.decode(&payload.signature_base64))
        .map_err(|e| WebError::PasskeyError(format!("Invalid signature: {}", e)))?;

    // 4. Verify signature cryptographically
    state.passkey_manager.verify_assertion(
        &cred.public_key,
        &auth_data,
        &client_data_json,
        &signature,
        &payload.challenge,
    )?;

    // 5. Update credential counter
    repo.update_fido2_counter(&cred.credential_id, cred.counter + 1)
        .await?;

    // 6. Issue authenticated session
    let user = repo
        .get_user_by_id(cred.user_id)
        .await?
        .ok_or_else(|| WebError::NotFound("User not found".to_string()))?;

    let token = generate_session_token();
    let token_hash = hash_session_token(&token);
    let expires_at = Utc::now() + chrono::Duration::days(14);
    repo.create_user_session(user.id, &token_hash, expires_at, None, None)
        .await?;

    let cookie = build_session_cookie(&token, 14 * 86400);
    let body = Json(AuthResponse { user, token });

    Ok(([(header::SET_COOKIE, cookie)], body).into_response())
}
