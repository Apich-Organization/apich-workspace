use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

/// Registered `OAuth2` client application.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OAuthClient {
    /// Unique public client identifier.
    pub client_id: String,
    /// Secure hash of client secret for confidential clients.
    pub client_secret_hash: Option<String>,
    /// Human-readable application name.
    pub name: String,
    /// Allowed `OAuth2` redirect URIs.
    pub redirect_uris: Vec<String>,
    /// Whether this is a confidential client requiring client authentication.
    pub is_confidential: bool,
    /// Registration timestamp.
    pub created_at: DateTime<Utc>,
}

/// Data transfer object for registering a new `OAuth2` client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateOAuthClientDto {
    /// Public client identifier.
    pub client_id: String,
    /// Optional hashed client secret.
    pub client_secret_hash: Option<String>,
    /// Client application name.
    pub name: String,
    /// Whitelisted redirect URIs.
    pub redirect_uris: Vec<String>,
    /// Confidential client flag.
    pub is_confidential: bool,
}

/// Transient `OAuth2` authorization code issued during the authorization code grant flow.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OAuthAuthCode {
    /// Authorization code string.
    pub code: String,
    /// Requesting client ID.
    pub client_id: String,
    /// Authenticated user ID who granted authorization.
    pub user_id: Uuid,
    /// Redirect URI supplied in authorization request.
    pub redirect_uri: String,
    /// Granted OAuth scopes.
    pub scope: String,
    /// Expiration timestamp of the code.
    pub expires_at: DateTime<Utc>,
    /// Issuance timestamp.
    pub created_at: DateTime<Utc>,
}

/// Standard `OpenID` Connect ID token JWT claims.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcClaims {
    /// Token issuer URL.
    pub iss: String,
    /// Subject identifier (user ID).
    pub sub: String,
    /// Audience (client ID).
    pub aud: String,
    /// Expiration time (UNIX epoch seconds).
    pub exp: i64,
    /// Issued-at time (UNIX epoch seconds).
    pub iat: i64,
    /// Time when end-user authentication occurred (UNIX epoch seconds).
    pub auth_time: i64,
    /// End-user's full name.
    pub name: String,
    /// End-user's email address.
    pub email: String,
    /// End-user's preferred username.
    pub preferred_username: String,
    /// Whether end-user's email has been verified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_verified: Option<bool>,
    /// User roles granted for access control.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub roles: Vec<String>,
}

/// `OpenID` Connect provider configuration discovery document metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcDiscovery {
    /// Issuer identifier URL.
    pub issuer: String,
    /// `OAuth2` authorization endpoint URL.
    pub authorization_endpoint: String,
    /// `OAuth2` token endpoint URL.
    pub token_endpoint: String,
    /// `UserInfo` endpoint URL.
    pub userinfo_endpoint: String,
    /// JSON Web Key Set URL.
    pub jwks_uri: String,
    /// List of supported response type values.
    pub response_types_supported: Vec<String>,
    /// List of supported subject type values.
    pub subject_types_supported: Vec<String>,
    /// List of supported ID token signing algorithms.
    pub id_token_signing_alg_values_supported: Vec<String>,
    /// List of supported `OAuth2` scopes.
    pub scopes_supported: Vec<String>,
    /// Supported client authentication methods at token endpoint.
    pub token_endpoint_auth_methods_supported: Vec<String>,
    /// List of supported `OpenID` claims.
    pub claims_supported: Vec<String>,
}
