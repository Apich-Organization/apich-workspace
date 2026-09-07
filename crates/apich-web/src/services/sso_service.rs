use crate::error::{WebError, WebResult};
use apich_db::{Database, OidcClaims, OidcDiscovery, User};
use base64::prelude::*;
use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct SsoService {
    db: Arc<Database>,
    issuer_url: String,
    jwt_secret: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub id_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JwksKey {
    pub kty: String,
    pub alg: String,
    pub r#use: String,
    pub kid: String,
    pub n: Option<String>,
    pub e: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JwksResponse {
    pub keys: Vec<JwksKey>,
}

impl SsoService {
    pub fn new(db: Arc<Database>, issuer_url: String, jwt_secret: String) -> Self {
        Self {
            db,
            issuer_url,
            jwt_secret,
        }
    }

    /// Return OIDC OpenID Provider Configuration discovery document
    pub fn get_discovery(&self) -> OidcDiscovery {
        OidcDiscovery {
            issuer: self.issuer_url.clone(),
            authorization_endpoint: format!("{}/oauth/authorize", self.issuer_url),
            token_endpoint: format!("{}/oauth/token", self.issuer_url),
            userinfo_endpoint: format!("{}/oauth/userinfo", self.issuer_url),
            jwks_uri: format!("{}/oauth/jwks.json", self.issuer_url),
            response_types_supported: vec!["code".to_string(), "token".to_string(), "id_token".to_string()],
            subject_types_supported: vec!["public".to_string()],
            id_token_signing_alg_values_supported: vec!["HS256".to_string()],
            scopes_supported: vec!["openid".to_string(), "profile".to_string(), "email".to_string()],
            token_endpoint_auth_methods_supported: vec!["client_secret_post".to_string(), "client_secret_basic".to_string()],
            claims_supported: vec![
                "sub".to_string(),
                "iss".to_string(),
                "aud".to_string(),
                "exp".to_string(),
                "iat".to_string(),
                "name".to_string(),
                "email".to_string(),
                "preferred_username".to_string(),
                "roles".to_string(),
            ],
        }
    }

    /// Issue an OAuth2 Authorization Code
    pub async fn issue_auth_code(
        &self,
        client_id: &str,
        user_id: Uuid,
        redirect_uri: &str,
        scope: &str,
    ) -> WebResult<String> {
        let repo = self.db.repository();
        let client = repo
            .get_oauth_client_by_id(client_id)
            .await?
            .ok_or_else(|| WebError::OAuthError("Client not found".to_string()))?;

        // Validate redirect_uri matches registered URIs
        if !client.redirect_uris.iter().any(|uri| uri == redirect_uri) {
            return Err(WebError::OAuthError("Invalid redirect URI".to_string()));
        }

        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        let code = BASE64_URL_SAFE_NO_PAD.encode(bytes);

        let expires_at = Utc::now() + chrono::Duration::minutes(5);
        repo.create_oauth_auth_code(&code, client_id, user_id, redirect_uri, scope, expires_at)
            .await?;

        Ok(code)
    }

    /// Exchange authorization code for access_token and id_token JWTs
    pub async fn exchange_code(
        &self,
        code: &str,
        client_id: &str,
        client_secret: Option<&str>,
        redirect_uri: &str,
    ) -> WebResult<TokenResponse> {
        let repo = self.db.repository();
        let client = repo
            .get_oauth_client_by_id(client_id)
            .await?
            .ok_or_else(|| WebError::OAuthError("Unknown client".to_string()))?;

        // If client is confidential, verify client_secret
        if client.is_confidential {
            let secret = client_secret.ok_or_else(|| WebError::OAuthError("Client secret required".to_string()))?;
            if let Some(hash) = &client.client_secret_hash {
                if !crate::auth::verify_password(secret, hash).unwrap_or(false) && secret != hash {
                    return Err(WebError::OAuthError("Invalid client secret".to_string()));
                }
            }
        }

        let auth_code = repo
            .consume_oauth_auth_code(code)
            .await?
            .ok_or_else(|| WebError::OAuthError("Invalid or expired authorization code".to_string()))?;

        if auth_code.client_id != client_id || auth_code.redirect_uri != redirect_uri {
            return Err(WebError::OAuthError("Authorization code parameter mismatch".to_string()));
        }

        let user = repo
            .get_user_by_id(auth_code.user_id)
            .await?
            .ok_or_else(|| WebError::NotFound("User not found".to_string()))?;

        let now = Utc::now().timestamp();
        let exp = now + 3600;

        // Generate ID Token claims
        let claims = OidcClaims {
            iss: self.issuer_url.clone(),
            sub: user.id.to_string(),
            aud: client_id.to_string(),
            exp,
            iat: now,
            auth_time: now,
            name: user.display_name.clone(),
            email: user.email.clone(),
            preferred_username: user.username.clone(),
            email_verified: Some(true),
            roles: vec![user.role.as_str().to_string()],
        };

        let id_token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|e| WebError::Internal(format!("JWT encode error: {}", e)))?;

        // For access_token, we can issue an opaque token or signed JWT
        let access_token = id_token.clone();

        Ok(TokenResponse {
            access_token,
            token_type: "Bearer".to_string(),
            expires_in: 3600,
            id_token,
            refresh_token: None,
        })
    }

    /// UserInfo endpoint claim resolver
    pub async fn get_userinfo(&self, user_id: Uuid) -> WebResult<User> {
        let repo = self.db.repository();
        let user = repo
            .get_user_by_id(user_id)
            .await?
            .ok_or_else(|| WebError::NotFound("User not found".to_string()))?;

        Ok(user)
    }

    /// Validate and decode a signed JWT access token or ID token
    pub fn validate_jwt(&self, token: &str) -> WebResult<OidcClaims> {
        let mut validation = Validation::default();
        validation.set_issuer(&[&self.issuer_url]);
        validation.validate_aud = false;

        let data = decode::<OidcClaims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &validation,
        )
        .map_err(|_| WebError::Unauthorized)?;

        Ok(data.claims)
    }
}
