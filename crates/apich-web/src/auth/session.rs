use crate::error::WebError;
use crate::state::AppState;
use apich_db::User;
use axum::async_trait;
use axum::extract::FromRef;
use axum::extract::FromRequestParts;
use axum::http::header;
use axum::http::request::Parts;
use base64::prelude::*;
use rand::RngCore;
use sha2::Digest;
use sha2::Sha256;

/// Name of the HTTP cookie used to store the user authentication session token.
pub const SESSION_COOKIE_NAME: &str = "apich_session";

/// Generate a cryptographically strong 32-byte session token encoded in `Base64URL`
#[must_use]
pub fn generate_session_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    BASE64_URL_SAFE_NO_PAD.encode(bytes)
}

/// Compute SHA-256 hash of a session token for secure database storage
#[must_use]
pub fn hash_session_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

/// Helper to format Set-Cookie header for session
#[must_use]
pub fn build_session_cookie(
    token: &str,
    max_age_secs: i64,
) -> String {
    format!("{SESSION_COOKIE_NAME}={token}; Path=/; Max-Age={max_age_secs}; HttpOnly; SameSite=Lax")
}

/// Helper to clear session cookie
#[must_use]
pub fn build_clear_cookie() -> String {
    format!("{SESSION_COOKIE_NAME}=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax")
}

/// Authenticated user extracted from session cookie or Authorization Bearer header
#[derive(Debug, Clone)]
pub struct AuthUser(pub User);

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = WebError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);

        // 1. Try Authorization header: Bearer <token>
        if let Some(auth_header) = parts.headers.get(header::AUTHORIZATION) {
            if let Ok(auth_str) = auth_header.to_str() {
                if let Some(raw_token) = auth_str.strip_prefix("Bearer ") {
                    let token = raw_token.trim();
                    if token.contains('.') {
                        if let Ok(claims) = app_state.sso_service.validate_jwt(token) {
                            if let Ok(user_id) = uuid::Uuid::parse_str(&claims.sub) {
                                let repo = app_state.db.repository();
                                if let Ok(Some(user)) = repo.get_user_by_id(user_id).await {
                                    if user.is_active {
                                        return Ok(Self(user));
                                    }
                                    return Err(WebError::Unauthorized);
                                }
                            }
                        }
                    }

                    let token_hash = hash_session_token(token);
                    let repo = app_state.db.repository();
                    if let Ok(Some(user)) = repo.get_user_by_session_token_hash(&token_hash).await {
                        if user.is_active {
                            return Ok(Self(user));
                        }
                        return Err(WebError::Unauthorized);
                    }
                    // Personal access tokens are hashed with the same scheme as session tokens
                    // (see `handlers::generate_pat`), so a PAT presented as a Bearer token works too.
                    if let Ok(Some(user)) = repo.get_user_by_active_pat_hash(&token_hash).await {
                        if user.is_active {
                            return Ok(Self(user));
                        }
                        return Err(WebError::Unauthorized);
                    }
                }

                // 1b. Try Authorization: Basic <base64(username:password)> -- the form real Git
                // clients (and `curl -u`) send. The username is accepted but not checked; the
                // password is treated as a personal access token, matching how GitHub/GitLab's
                // own HTTPS git auth works.
                if let Some(raw_basic) = auth_str.strip_prefix("Basic ") {
                    if let Ok(decoded) = BASE64_STANDARD.decode(raw_basic.trim()) {
                        if let Ok(decoded_str) = String::from_utf8(decoded) {
                            if let Some((_username, password)) = decoded_str.split_once(':') {
                                let token_hash = hash_session_token(password);
                                let repo = app_state.db.repository();
                                if let Ok(Some(user)) =
                                    repo.get_user_by_active_pat_hash(&token_hash).await
                                {
                                    if user.is_active {
                                        return Ok(Self(user));
                                    }
                                    return Err(WebError::Unauthorized);
                                }
                            }
                        }
                    }
                }
            }
        }

        // 2. Try Cookie: apich_session=<token>
        if let Some(cookie_header) = parts.headers.get(header::COOKIE) {
            if let Ok(cookie_str) = cookie_header.to_str() {
                for cookie in cookie_str.split(';') {
                    let mut parts = cookie.trim().splitn(2, '=');
                    if let (Some(name), Some(val)) = (parts.next(), parts.next()) {
                        if name == SESSION_COOKIE_NAME {
                            let token_hash = hash_session_token(val);
                            let repo = app_state.db.repository();
                            if let Ok(Some(user)) =
                                repo.get_user_by_session_token_hash(&token_hash).await
                            {
                                if user.is_active {
                                    return Ok(Self(user));
                                }
                                return Err(WebError::Unauthorized);
                            }
                        }
                    }
                }
            }
        }

        Err(WebError::Unauthorized)
    }
}

/// Extractor that requires platform admin status
#[derive(Debug, Clone)]
pub struct RequirePlatformAdmin(pub User);

#[async_trait]
impl<S> FromRequestParts<S> for RequirePlatformAdmin
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = WebError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let AuthUser(user) = AuthUser::from_request_parts(parts, state).await?;
        if user.is_platform_admin || user.role == apich_db::UserRole::Admin {
            Ok(Self(user))
        } else {
            Err(WebError::Forbidden(
                "Platform administrator privileges required".to_string(),
            ))
        }
    }
}
