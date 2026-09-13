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
use webauthn_rs::prelude::*;

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
    pub confirm_password: Option<String>,
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

    if let Some(ref confirm) = payload.confirm_password {
        if confirm != &payload.password {
            return Err(WebError::BadRequest("Passwords do not match".to_string()));
        }
    }

    // Verify registration mode and invitation code
    let has_invite = payload
        .invite_token
        .as_deref()
        .is_some_and(|t| !t.trim().is_empty());

    let invite_to_mark = match settings.registration_mode.as_str() {
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
                WebError::BadRequest("Invalid, expired, or exhausted invitation code".to_string())
            })?;

            if let Some(ref req_email) = invite.email {
                if !req_email.trim().is_empty()
                    && req_email.to_lowercase() != payload.email.to_lowercase()
                {
                    return Err(WebError::BadRequest(
                        "Registration email does not match invitation recipient".to_string(),
                    ));
                }
            }
            Some(invite)
        },
        | _ => {
            if has_invite {
                let token = payload.invite_token.as_deref().unwrap_or_default();
                let invite = repo.get_invitation_by_token(token).await?.ok_or_else(|| {
                    WebError::BadRequest(
                        "Invalid, expired, or exhausted invitation code".to_string(),
                    )
                })?;
                if let Some(ref req_email) = invite.email {
                    if !req_email.trim().is_empty()
                        && req_email.to_lowercase() != payload.email.to_lowercase()
                    {
                        return Err(WebError::BadRequest(
                            "Registration email does not match invitation recipient".to_string(),
                        ));
                    }
                }
                Some(invite)
            } else {
                None
            }
        },
    };

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

    if let Some(invite) = invite_to_mark {
        repo.mark_invitation_used(&invite.token).await?;
    }

    // Send welcome email
    let _ = state
        .mailer
        .send_welcome_email(
            &settings,
            &user.email,
            &user.username,
            &user.display_name,
            &state.base_url,
        )
        .await;

    // Create initial session
    let token = generate_session_token();
    let token_hash = hash_session_token(&token);
    let expires_at = Utc::now()
        .checked_add_signed(chrono::Duration::days(14))
        .unwrap_or_else(Utc::now);
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
    let expires_at = Utc::now()
        .checked_add_signed(chrono::Duration::days(14))
        .unwrap_or_else(Utc::now);
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
    pub credential_id: Option<String>,
    pub public_key_base64: Option<String>,
    pub device_name: Option<String>,
    pub credential_json: Option<String>,
}

async fn passkey_register_start(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> WebResult<Json<serde_json::Value>> {
    let repo = state.db.repository();
    let existing_creds = repo.get_fido2_credentials_by_user(user.id).await?;
    let exclude_credentials: Vec<CredentialID> = existing_creds
        .iter()
        .filter_map(|c| {
            c.passkey_json
                .as_deref()
                .and_then(|j| serde_json::from_str::<Passkey>(j).ok())
        })
        .map(|pk| pk.cred_id().clone())
        .collect();

    let ccr = state.passkey_manager.start_registration(
        user.id,
        &user.username,
        &user.display_name,
        if exclude_credentials.is_empty() {
            None
        } else {
            Some(exclude_credentials)
        },
    )?;

    let mut val = serde_json::to_value(&ccr)
        .map_err(|e| WebError::Internal(format!("Serialization error: {e}")))?;
    if let Some(obj) = val.as_object_mut() {
        let challenge_str = BASE64_URL_SAFE_NO_PAD.encode(&ccr.public_key.challenge);
        obj.insert("challenge".to_string(), serde_json::Value::String(challenge_str));
        obj.insert(
            "rp".to_string(),
            json!({
                "name": ccr.public_key.rp.name,
                "id": ccr.public_key.rp.id,
            }),
        );
        obj.insert(
            "user".to_string(),
            json!({
                "id": user.id.to_string(),
                "name": user.username,
                "displayName": user.display_name,
            }),
        );
    }
    Ok(Json(val))
}

async fn passkey_register_finish(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<PasskeyRegisterFinishRequest>,
) -> WebResult<Json<serde_json::Value>> {
    let repo = state.db.repository();
    let device_name = payload
        .device_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("Passkey");

    if let Some(cred_json) = payload.credential_json {
        let reg: RegisterPublicKeyCredential = serde_json::from_str(&cred_json)
            .map_err(|e| WebError::PasskeyError(format!("Invalid credential JSON: {e}")))?;

        let passkey = state.passkey_manager.finish_registration(user.id, &reg)?;
        let passkey_json = serde_json::to_string(&passkey)
            .map_err(|e| WebError::Internal(format!("Failed to serialize passkey: {e}")))?;
        let cred_id_str = BASE64_URL_SAFE_NO_PAD.encode(passkey.cred_id());

        repo.save_fido2_credential(
            user.id,
            &cred_id_str,
            passkey.cred_id(),
            0,
            device_name,
            None,
            Some(&passkey_json),
        )
        .await?;
    } else if let (Some(cred_id), Some(pk_b64)) = (payload.credential_id, payload.public_key_base64) {
        let public_key_bytes = BASE64_STANDARD
            .decode(&pk_b64)
            .or_else(|_| BASE64_URL_SAFE_NO_PAD.decode(&pk_b64))
            .map_err(|e| WebError::PasskeyError(format!("Invalid public key base64: {e}")))?;

        repo.save_fido2_credential(
            user.id,
            &cred_id,
            &public_key_bytes,
            0,
            device_name,
            None,
            None,
        )
        .await?;
    } else {
        return Err(WebError::BadRequest("Missing credential data".to_string()));
    }

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
    pub credential_json: Option<String>,
    pub credential_id: Option<String>,
    pub challenge: Option<String>,
    pub client_data_json_base64: Option<String>,
    pub auth_data_base64: Option<String>,
    pub signature_base64: Option<String>,
}

async fn passkey_auth_start(
    State(state): State<AppState>,
) -> WebResult<Json<serde_json::Value>> {
    let rcr = state.passkey_manager.start_discoverable_authentication()?;
    let mut val = serde_json::to_value(&rcr)
        .map_err(|e| WebError::Internal(format!("Serialization error: {e}")))?;
    if let Some(obj) = val.as_object_mut() {
        let challenge_str = BASE64_URL_SAFE_NO_PAD.encode(&rcr.public_key.challenge);
        obj.insert("challenge".to_string(), serde_json::Value::String(challenge_str));
        obj.insert("timeout".to_string(), json!(60000));
    }
    Ok(Json(val))
}

async fn passkey_auth_finish(
    State(state): State<AppState>,
    Json(payload): Json<PasskeyAuthFinishRequest>,
) -> WebResult<Response> {
    let repo = state.db.repository();

    let (user_id, counter_to_update) = if let Some(cred_json) = payload.credential_json {
        let cred: PublicKeyCredential = serde_json::from_str(&cred_json)
            .map_err(|e| WebError::PasskeyError(format!("Invalid credential JSON: {e}")))?;

        let user_id = state.passkey_manager.identify_credential(&cred)?;

        let db_creds = repo.get_fido2_credentials_by_user(user_id).await?;
        let passkeys: Vec<(String, Passkey)> = db_creds
            .into_iter()
            .filter_map(|c| {
                c.passkey_json.as_deref().and_then(|j| {
                    serde_json::from_str::<Passkey>(j).ok().map(|pk| (c.credential_id, pk))
                })
            })
            .collect();

        if passkeys.is_empty() {
            return Err(WebError::PasskeyError("No passkeys registered for that account".to_string()));
        }

        let discoverable_keys: Vec<DiscoverableKey> = passkeys
            .iter()
            .map(|(_, pk)| DiscoverableKey::from(pk))
            .collect();

        let auth_result = state.passkey_manager.finish_discoverable_authentication(&cred, &discoverable_keys)?;

        for (cred_id, mut pk) in passkeys {
            if pk.update_credential(&auth_result).is_some() {
                if let Ok(updated_json) = serde_json::to_string(&pk) {
                    let _ = repo
                        .update_fido2_passkey(&cred_id, i64::from(auth_result.counter()), &updated_json)
                        .await;
                }
                break;
            }
        }

        (user_id, None)
    } else if let (Some(cred_id), Some(challenge), Some(cdj_b64), Some(ad_b64), Some(sig_b64)) = (
        payload.credential_id,
        payload.challenge,
        payload.client_data_json_base64,
        payload.auth_data_base64,
        payload.signature_base64,
    ) {
        let _ = state.passkey_manager.verify_and_consume_challenge(&challenge)?;

        let cred = repo
            .get_fido2_credential_by_id(&cred_id)
            .await?
            .ok_or_else(|| WebError::PasskeyError("Unknown security credential".to_string()))?;

        let client_data_json = BASE64_STANDARD
            .decode(&cdj_b64)
            .or_else(|_| BASE64_URL_SAFE_NO_PAD.decode(&cdj_b64))
            .map_err(|e| WebError::PasskeyError(format!("Invalid clientDataJSON: {e}")))?;

        let auth_data = BASE64_STANDARD
            .decode(&ad_b64)
            .or_else(|_| BASE64_URL_SAFE_NO_PAD.decode(&ad_b64))
            .map_err(|e| WebError::PasskeyError(format!("Invalid authData: {e}")))?;

        let signature = BASE64_STANDARD
            .decode(&sig_b64)
            .or_else(|_| BASE64_URL_SAFE_NO_PAD.decode(&sig_b64))
            .map_err(|e| WebError::PasskeyError(format!("Invalid signature: {e}")))?;

        state.passkey_manager.verify_assertion(
            &cred.public_key,
            &auth_data,
            &client_data_json,
            &signature,
            &challenge,
        )?;

        (cred.user_id, Some((cred.credential_id, cred.counter.saturating_add(1))))
    } else {
        return Err(WebError::BadRequest("Missing credential data".to_string()));
    };

    if let Some((cid, new_counter)) = counter_to_update {
        repo.update_fido2_counter(&cid, new_counter).await?;
    }

    let user = repo
        .get_user_by_id(user_id)
        .await?
        .ok_or_else(|| WebError::NotFound("User not found".to_string()))?;

    let token = generate_session_token();
    let token_hash = hash_session_token(&token);
    let expires_at = Utc::now()
        .checked_add_signed(chrono::Duration::days(14))
        .unwrap_or_else(Utc::now);
    repo.create_user_session(user.id, &token_hash, expires_at, None, None)
        .await?;

    let cookie = build_session_cookie(&token, 14 * 86400);
    let body = Json(AuthResponse { user, token });

    Ok(([(header::SET_COOKIE, cookie)], body).into_response())
}
