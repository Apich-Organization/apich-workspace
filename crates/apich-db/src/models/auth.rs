use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

/// Active web or API session for an authenticated user.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserSession {
    /// Session unique identifier.
    pub id: Uuid,
    /// Associated user ID.
    pub user_id: Uuid,
    /// Secure hash of the session token.
    pub token_hash: String,
    /// User agent string of the client device.
    pub user_agent: Option<String>,
    /// Client IP address at session creation.
    pub ip_address: Option<String>,
    /// Expiration timestamp of the session.
    pub expires_at: DateTime<Utc>,
    /// Creation timestamp of the session.
    pub created_at: DateTime<Utc>,
}

/// Registered `WebAuthn` / FIDO2 passkey credential for passwordless login.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Fido2Credential {
    /// Unique identifier of the passkey credential record.
    pub id: Uuid,
    /// ID of the user owning this credential.
    pub user_id: Uuid,
    /// Base64-encoded raw credential ID from the authenticator.
    pub credential_id: String,
    /// CBOR or COSE public key bytes.
    pub public_key: Vec<u8>,
    /// Signature counter tracking passkey clone detection.
    pub counter: i64,
    /// Human-readable device label given by the user.
    pub device_name: String,
    /// Authenticator Attestation GUID identifying the authenticator model.
    pub aaguid: Option<Vec<u8>>,
    /// Registration timestamp of the credential.
    pub created_at: DateTime<Utc>,
    /// Timestamp when this passkey was last used to authenticate.
    pub last_used_at: Option<DateTime<Utc>>,
}

/// Platform user registration policy mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RegistrationMode {
    /// Anyone can freely register a new account.
    Open,
    /// Registration requires an invitation link or code.
    #[default]
    InviteOnly,
    /// Only platform administrators can provision new accounts.
    AdminOnly,
}

impl RegistrationMode {
    /// Returns the string representation of the registration mode.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            | Self::Open => "open",
            | Self::InviteOnly => "invite_only",
            | Self::AdminOnly => "admin_only",
        }
    }
}

impl std::str::FromStr for RegistrationMode {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s {
            | "open" => Self::Open,
            | "admin_only" => Self::AdminOnly,
            | _ => Self::InviteOnly,
        })
    }
}

/// Global system settings and SMTP email dispatch configuration.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SystemSettings {
    /// Singleton record identifier.
    pub id: i32,
    /// Current registration policy ("open", "`invite_only`", or "`admin_only`").
    pub registration_mode: String,
    /// SMTP server hostname.
    pub smtp_host: Option<String>,
    /// SMTP server port.
    pub smtp_port: Option<i32>,
    /// SMTP username.
    pub smtp_username: Option<String>,
    /// SMTP password or app secret.
    pub smtp_password: Option<String>,
    /// "From" email address.
    pub smtp_from_email: Option<String>,
    /// "From" sender display name.
    pub smtp_from_name: Option<String>,
    /// Whether to establish TLS encryption for SMTP connections.
    pub smtp_use_tls: bool,
    /// Whether outbound email sending is enabled.
    pub smtp_enabled: bool,
    /// Last update timestamp of system settings.
    pub updated_at: DateTime<Utc>,
}

impl Default for SystemSettings {
    fn default() -> Self {
        Self {
            id: 1,
            registration_mode: "invite_only".to_string(),
            smtp_host: None,
            smtp_port: None,
            smtp_username: None,
            smtp_password: None,
            smtp_from_email: None,
            smtp_from_name: None,
            smtp_use_tls: false,
            smtp_enabled: false,
            updated_at: Utc::now(),
        }
    }
}

/// Data transfer object for updating global system settings.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateSystemSettingsDto {
    /// Updated registration mode.
    pub registration_mode: Option<String>,
    /// Updated SMTP hostname.
    pub smtp_host: Option<String>,
    /// Updated SMTP port.
    pub smtp_port: Option<i32>,
    /// Updated SMTP username.
    pub smtp_username: Option<String>,
    /// Updated SMTP password.
    pub smtp_password: Option<String>,
    /// Updated "From" email address.
    pub smtp_from_email: Option<String>,
    /// Updated "From" sender name.
    pub smtp_from_name: Option<String>,
    /// Updated TLS setting.
    pub smtp_use_tls: Option<bool>,
    /// Updated enable/disable flag for email dispatch.
    pub smtp_enabled: Option<bool>,
}

/// Pending or redeemed invitation to register or join an organization.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Invitation {
    /// Unique invitation identifier.
    pub id: Uuid,
    /// Secure random token or custom code passed via invitation URL or entered directly.
    pub token: String,
    /// Optional invited email address (if restricted to a specific recipient).
    pub email: Option<String>,
    /// Organization to join upon acceptance.
    pub org_id: Option<Uuid>,
    /// Team to join upon acceptance.
    pub team_id: Option<Uuid>,
    /// Default role assigned upon acceptance.
    pub role: String,
    /// ID of the user who issued the invitation.
    pub inviter_id: Option<Uuid>,
    /// Maximum number of times this invitation code can be redeemed.
    pub max_uses: i32,
    /// Number of times this invitation code has been redeemed.
    pub used_count: i32,
    /// Expiration timestamp of the invitation.
    pub expires_at: DateTime<Utc>,
    /// Timestamp when the invitation was last redeemed, if used.
    pub used_at: Option<DateTime<Utc>>,
    /// Creation timestamp of the invitation.
    pub created_at: DateTime<Utc>,
}

/// Data transfer object for creating a new invitation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInvitationDto {
    /// Optional custom invitation token/code. Generated if None.
    pub token: Option<String>,
    /// Optional recipient email address. If None, code can be redeemed by any user.
    pub email: Option<String>,
    /// Target organization ID.
    pub org_id: Option<Uuid>,
    /// Target team ID.
    pub team_id: Option<Uuid>,
    /// Assigned role name.
    pub role: Option<String>,
    /// ID of inviting user.
    pub inviter_id: Option<Uuid>,
    /// Maximum number of uses allowed (defaults to 1).
    pub max_uses: Option<i32>,
    /// Expiration timestamp.
    pub expires_at: DateTime<Utc>,
}

/// A personal access token record.
///
/// The plaintext token is shown to the user exactly once, at
/// creation time, and never stored -- only `token_hash` (SHA-256) is persisted, matching
/// `user_sessions.token_hash`'s existing pattern.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PersonalAccessToken {
    /// Unique token identifier.
    pub id: Uuid,
    /// User who owns this token.
    pub user_id: Uuid,
    /// Descriptive name for the token.
    pub name: String,
    /// SHA-256 hash of the secret token.
    pub token_hash: String,
    /// Visible prefix of the token for identification in UI.
    pub token_prefix: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Timestamp of most recent use.
    pub last_used_at: Option<DateTime<Utc>>,
    /// Optional expiration timestamp.
    pub expires_at: Option<DateTime<Utc>>,
    /// Revocation timestamp, if revoked.
    pub revoked_at: Option<DateTime<Utc>>,
}

impl PersonalAccessToken {
    /// Returns true if the token is neither revoked nor expired.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.revoked_at.is_none() && self.expires_at.is_none_or(|e| e > Utc::now())
    }
}

/// A user's uploaded SSH public key. Storage/identity only -- there is no SSH transport server
/// yet, so this key is not currently usable to authenticate a `git+ssh://` or `ssh://` connection.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SshPublicKey {
    /// Unique SSH key identifier.
    pub id: Uuid,
    /// User who owns this key.
    pub user_id: Uuid,
    /// Human-readable label for this key.
    pub name: String,
    /// Key type algorithm (e.g. "ssh-ed25519" or "ssh-rsa").
    pub key_type: String,
    /// Base64 encoded public key data.
    pub public_key: String,
    /// SHA-256 fingerprint of the key.
    pub fingerprint: String,
    /// Upload timestamp.
    pub created_at: DateTime<Utc>,
}

/// A user's registered GPG public key, used to verify the `gpg_signature` on apich-vcs snapshots
/// they authored (see `apich_vcs::gpg::verify_signature`).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GpgPublicKey {
    /// Unique GPG key identifier.
    pub id: Uuid,
    /// User who owns this GPG key.
    pub user_id: Uuid,
    /// Label or user ID associated with this key.
    pub name: String,
    /// Armored public key block.
    pub public_key: String,
    /// Hex fingerprint of the GPG key.
    pub fingerprint: String,
    /// Upload timestamp.
    pub created_at: DateTime<Utc>,
}
