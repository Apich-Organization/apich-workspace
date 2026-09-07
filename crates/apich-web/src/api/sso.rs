use crate::{
    auth::AuthUser,
    error::{WebError, WebResult},
    services::TokenResponse,
    state::AppState,
};
use apich_db::OidcDiscovery;
use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
    Form, Json, Router,
};
use serde::Deserialize;
use serde_json::json;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/.well-known/openid-configuration", get(openid_configuration))
        .route("/oauth/jwks.json", get(jwks))
        .route("/oauth/authorize", get(authorize).post(authorize_post))
        .route("/oauth/token", post(token))
        .route("/oauth/userinfo", get(userinfo))
}

#[derive(Debug, Deserialize)]
pub struct AuthorizeQuery {
    pub response_type: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: Option<String>,
    pub state: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TokenRequest {
    pub grant_type: String,
    pub code: String,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub redirect_uri: String,
}

async fn openid_configuration(State(state): State<AppState>) -> Json<OidcDiscovery> {
    Json(state.sso_service.get_discovery())
}

async fn jwks() -> Json<serde_json::Value> {
    Json(json!({
        "keys": [
            {
                "kty": "oct",
                "alg": "HS256",
                "use": "sig",
                "kid": "apich-default-key"
            }
        ]
    }))
}

async fn authorize(
    auth_user: Option<AuthUser>,
    State(state): State<AppState>,
    Query(params): Query<AuthorizeQuery>,
) -> WebResult<Response> {
    if params.response_type != "code" {
        return Err(WebError::OAuthError("Unsupported response_type; must be 'code'".to_string()));
    }

    // If user is not authenticated, redirect to login page preserving query params
    let AuthUser(user) = match auth_user {
        Some(u) => u,
        None => {
            let return_to = format!(
                "/oauth/authorize?response_type={}&client_id={}&redirect_uri={}&state={}",
                params.response_type,
                params.client_id,
                urlencoding::encode(&params.redirect_uri),
                urlencoding::encode(params.state.as_deref().unwrap_or(""))
            );
            return Ok(Redirect::temporary(&format!("/login?return_to={}", urlencoding::encode(&return_to))).into_response());
        }
    };

    let scope = params.scope.unwrap_or_else(|| "openid profile email".to_string());
    let code = state
        .sso_service
        .issue_auth_code(&params.client_id, user.id, &params.redirect_uri, &scope)
        .await?;

    let redirect_url = match &params.state {
        Some(s) => format!(
            "{}?code={}&state={}",
            params.redirect_uri,
            code,
            urlencoding::encode(s)
        ),
        None => format!("{}?code={}", params.redirect_uri, code),
    };

    Ok(Redirect::temporary(&redirect_url).into_response())
}

async fn authorize_post(
    auth_user: Option<AuthUser>,
    state: State<AppState>,
    Form(params): Form<AuthorizeQuery>,
) -> WebResult<Response> {
    authorize(auth_user, state, Query(params)).await
}

async fn token(
    State(state): State<AppState>,
    Form(payload): Form<TokenRequest>,
) -> WebResult<Json<TokenResponse>> {
    if payload.grant_type != "authorization_code" {
        return Err(WebError::OAuthError("Unsupported grant_type; must be 'authorization_code'".to_string()));
    }

    let tokens = state
        .sso_service
        .exchange_code(
            &payload.code,
            &payload.client_id,
            payload.client_secret.as_deref(),
            &payload.redirect_uri,
        )
        .await?;

    Ok(Json(tokens))
}

async fn userinfo(
    AuthUser(user): AuthUser,
    State(state): State<AppState>,
) -> WebResult<Json<serde_json::Value>> {
    let repo = state.db.repository();
    let orgs = repo.list_organizations_for_user(user.id).await.unwrap_or_default();

    Ok(Json(json!({
        "sub": user.id.to_string(),
        "name": user.display_name,
        "preferred_username": user.username,
        "email": user.email,
        "email_verified": true,
        "role": user.role.as_str(),
        "is_platform_admin": user.is_platform_admin,
        "organizations": orgs.into_iter().map(|o| json!({ "id": o.id, "slug": o.slug, "name": o.name })).collect::<Vec<_>>(),
    })))
}
