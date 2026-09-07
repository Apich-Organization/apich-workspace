pub mod admin;
pub mod auth;
pub mod identity;
pub mod projects;
pub mod sso;

use crate::state::AppState;
use axum::Router;

pub fn build_api_router() -> Router<AppState> {
    Router::new()
        .nest("/api/auth", auth::router())
        .nest("/api", identity::router())
        .nest("/api", projects::router())
        .nest("/api/admin", admin::router())
        .merge(sso::router())
}
