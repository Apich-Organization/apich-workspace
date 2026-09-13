/// `WebAuthn` / FIDO2 passkey registration and authentication.
pub mod passkey;
/// Password hashing and verification via Argon2id.
pub mod password;
/// HTTP session cookies and authentication extractors.
pub mod session;

/// Two-Factor Authentication (TOTP / RFC 6238).
pub mod totp;

pub use passkey::PasskeyManager;
pub use password::hash_password;
pub use password::verify_password;
pub use session::build_clear_cookie;
pub use session::build_clear_2fa_cookie;
pub use session::build_session_cookie;
pub use session::build_2fa_cookie;
pub use session::generate_session_token;
pub use session::hash_session_token;
pub use session::AuthUser;
pub use session::RequirePlatformAdmin;
pub use session::SESSION_COOKIE_NAME;
pub use session::TWO_FACTOR_COOKIE_NAME;
pub use totp::generate_totp_setup;
pub use totp::verify_totp;
pub use totp::TotpSetupData;
