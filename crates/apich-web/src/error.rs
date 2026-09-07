use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

pub type WebResult<T> = Result<T, WebError>;

#[derive(Debug, Error)]
pub enum WebError {
    #[error("Database error: {0}")]
    Database(#[from] apich_db::DbError),

    #[error("SQLx query error: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("Sandbox error: {0}")]
    Sandbox(#[from] apich_sandbox::SandboxError),

    #[error("VCS error: {0}")]
    Vcs(#[from] apich_vcs::VcsError),

    #[error("Authentication required")]
    Unauthorized,

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Permission denied: {0}")]
    Forbidden(String),

    #[error("Entity not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("FIDO2 / WebAuthn verification failed: {0}")]
    PasskeyError(String),

    #[error("OAuth2 error: {0}")]
    OAuthError(String),

    #[error("Internal server error: {0}")]
    Internal(String),
}

impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            WebError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            WebError::InvalidCredentials => (StatusCode::UNAUTHORIZED, self.to_string()),
            WebError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg.clone()),
            WebError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            WebError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            WebError::Conflict(msg) => (StatusCode::CONFLICT, msg.clone()),
            WebError::PasskeyError(msg) => (StatusCode::BAD_REQUEST, format!("Passkey error: {}", msg)),
            WebError::OAuthError(msg) => (StatusCode::BAD_REQUEST, format!("OAuth error: {}", msg)),
            WebError::Database(err) => {
                tracing::error!(%err, "Database error");
                (StatusCode::INTERNAL_SERVER_ERROR, "A database error occurred".to_string())
            }
            WebError::Sqlx(err) => {
                tracing::error!(%err, "SQLx error");
                (StatusCode::INTERNAL_SERVER_ERROR, "A database error occurred".to_string())
            }
            WebError::Sandbox(err) => {
                tracing::error!(%err, "Sandbox error");
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Sandbox error: {}", err))
            }
            WebError::Vcs(err) => {
                tracing::error!(%err, "VCS error");
                (StatusCode::INTERNAL_SERVER_ERROR, format!("VCS error: {}", err))
            }
            WebError::Internal(err) => {
                tracing::error!(%err, "Internal server error");
                (StatusCode::INTERNAL_SERVER_ERROR, "An internal server error occurred".to_string())
            }
        };

        let body = Json(json!({
            "error": message,
            "status": status.as_u16(),
        }));

        (status, body).into_response()
    }
}
