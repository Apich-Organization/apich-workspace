use crate::error::WebError;
use crate::error::WebResult;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::PasswordHash;
use argon2::password_hash::PasswordHasher;
use argon2::password_hash::PasswordVerifier;
use argon2::password_hash::SaltString;
use argon2::Argon2;

/// Hash a plaintext password using Argon2id with random salt.
///
/// # Errors
///
/// Returns an error if the password could not be hashed.
pub fn hash_password(password: &str) -> WebResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| WebError::Internal(format!("Password hashing failed: {e}")))?
        .to_string();

    Ok(password_hash)
}

/// Verify a plaintext password against an Argon2id hash.
///
/// # Errors
///
/// Returns an error if the password hash cannot be verified due to internal argon2 errors.
pub fn verify_password(
    password: &str,
    password_hash: &str,
) -> WebResult<bool> {
    let Ok(parsed_hash) = PasswordHash::new(password_hash) else {
        return Ok(false);
    };

    let argon2 = Argon2::default();
    match argon2.verify_password(password.as_bytes(), &parsed_hash) {
        | Ok(()) => Ok(true),
        | Err(argon2::password_hash::Error::Password) => Ok(false),
        | Err(e) => {
            Err(WebError::Internal(format!(
                "Password verification error: {e}"
            )))
        },
    }
}
