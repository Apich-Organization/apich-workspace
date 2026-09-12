use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::Json;
use serde_json::json;
use thiserror::Error;

/// Result alias for operations returning a [`WebError`].
pub type WebResult<T> = Result<T, WebError>;

/// Top-level error type for the APICH web application and HTTP endpoints.
#[derive(Debug, Error)]
pub enum WebError {
    /// Underlying database operation error.
    #[error("Database error: {0}")]
    Database(#[from] apich_db::DbError),

    /// Direct SQL query error.
    #[error("SQLx query error: {0}")]
    Sqlx(#[from] sqlx::Error),

    /// Sandbox container execution error.
    #[error("Sandbox error: {0}")]
    Sandbox(#[from] apich_sandbox::SandboxError),

    /// Version control system operation error.
    #[error("VCS error: {0}")]
    Vcs(#[from] apich_vcs::VcsError),

    /// Unauthenticated request requiring login.
    #[error("Authentication required")]
    Unauthorized,

    /// Incorrect username, password, or token.
    #[error("Invalid credentials")]
    InvalidCredentials,

    /// Request rejected due to insufficient user role or permissions.
    #[error("Permission denied: {0}")]
    Forbidden(String),

    /// Requested resource was not found.
    #[error("Entity not found: {0}")]
    NotFound(String),

    /// Invalid request payload or parameter.
    #[error("Bad request: {0}")]
    BadRequest(String),

    /// Unique constraint violation or state conflict.
    #[error("Conflict: {0}")]
    Conflict(String),

    /// `WebAuthn` / FIDO2 credential validation failure.
    #[error("FIDO2 / WebAuthn verification failed: {0}")]
    PasskeyError(String),

    /// Single sign-on or `OAuth2` flow error.
    #[error("OAuth2 error: {0}")]
    OAuthError(String),

    /// Unclassified internal server error.
    #[error("Internal server error: {0}")]
    Internal(String),
}

impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            | Self::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            | Self::InvalidCredentials => (StatusCode::UNAUTHORIZED, self.to_string()),
            | Self::Forbidden(msg) => (StatusCode::FORBIDDEN, msg.clone()),
            | Self::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            | Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            | Self::Conflict(msg) => (StatusCode::CONFLICT, msg.clone()),
            | Self::PasskeyError(msg) => (StatusCode::BAD_REQUEST, format!("Passkey error: {msg}")),
            | Self::OAuthError(msg) => (StatusCode::BAD_REQUEST, format!("OAuth error: {msg}")),
            | Self::Database(err) => {
                tracing::error!(%err, "Database error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "A database error occurred".to_string(),
                )
            },
            | Self::Sqlx(err) => {
                tracing::error!(%err, "SQLx error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "A database error occurred".to_string(),
                )
            },
            | Self::Sandbox(err) => {
                tracing::error!(%err, "Sandbox error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Sandbox error: {err}"),
                )
            },
            | Self::Vcs(err) => {
                tracing::error!(%err, "VCS error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("VCS error: {err}"),
                )
            },
            | Self::Internal(err) => {
                tracing::error!(%err, "Internal server error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "An internal server error occurred".to_string(),
                )
            },
        };

        let body = Json(json!({
            "error": message,
            "status": status.as_u16(),
        }));

        (status, body).into_response()
    }
}
