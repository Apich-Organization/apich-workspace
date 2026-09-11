#![recursion_limit = "512"]

pub mod api;
pub mod app;
pub mod auth;
pub mod error;
pub mod mailer;
pub mod services;
pub mod state;
pub mod ui;

pub use error::WebError;
pub use error::WebResult;
pub use state::AppState;

async fn not_found() -> (axum::http::StatusCode, axum::response::Html<String>) {
    let i18n = ui::I18n::new(ui::Lang::default());
    let body = ui::views::render_page(
        "Not Found",
        r#"<div class="empty-state"><h2>404</h2><p>This page does not exist.</p><a href="/" class="btn btn-primary">Back to dashboard</a></div>"#,
        &i18n,
    );
    (
        axum::http::StatusCode::NOT_FOUND,
        axum::response::Html(body),
    )
}

/// Creates the top-level Axum router binding API endpoints and UI page routes.
/// Every page is rendered by a real Leptos component (`app::pages::*`), invoked directly
/// with live request data from the matching handler in `ui::handlers` -- SSR-only via
/// `RenderHtml::to_html()`, no client router/hydration/wasm bundle.
pub fn create_app(state: AppState) -> axum::Router {
    // Compiled island wasm+js bundle (see app::islands_pkg_dir / build_islands.sh). Served as
    // plain static files -- nothing here executes on the server, the browser fetches and runs it.
    let pkg_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/pkg");
    // `apich_islands.js`/`apich_islands_bg.wasm` keep the same filename across every rebuild (no
    // content hash in the name), and `ServeDir` alone sends no `Cache-Control` at all -- browsers
    // then fall back to heuristic caching and can keep serving a stale bundle from a much earlier
    // page load for a long time, silently running old island code against newly rendered HTML
    // (confirmed live: this is exactly what made the terminal and other islands look broken after
    // a server-side rebuild, even though a fresh browser profile picked up the new bundle fine).
    // `no-cache` still allows caching but forces revalidation against `Last-Modified` (which
    // `ServeDir` already sends) on every load, so a plain reload always gets the current bundle.
    let pkg_service = tower::ServiceBuilder::new()
        .layer(tower_http::set_header::SetResponseHeaderLayer::overriding(
            axum::http::header::CACHE_CONTROL,
            axum::http::HeaderValue::from_static("no-cache"),
        ))
        .service(tower_http::services::ServeDir::new(pkg_dir));
    axum::Router::new()
        .merge(api::build_api_router())
        .merge(ui::build_ui_router())
        .nest_service("/pkg", pkg_service)
        .fallback(not_found)
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .with_state(state)
}
