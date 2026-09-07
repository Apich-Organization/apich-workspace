#![recursion_limit = "512"]

pub mod api;
pub mod app;
pub mod auth;
pub mod error;
pub mod mailer;
pub mod services;
pub mod state;
pub mod ui;

pub use error::{WebError, WebResult};
pub use state::AppState;

/// Creates the top-level Axum router binding API endpoints, UI page routes and Leptos SSR fallback
pub fn create_app(state: AppState) -> axum::Router {
    axum::Router::new()
        .merge(api::build_api_router())
        .merge(ui::build_ui_router())
        .fallback(leptos_axum::render_app_to_stream(app::App))
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .with_state(state)
}

