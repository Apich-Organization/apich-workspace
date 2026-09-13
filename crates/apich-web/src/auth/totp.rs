//! Two-Factor Authentication (TOTP / RFC 6238) helpers.
//!
//! Provides secret generation, authenticator QR code rendering, and token validation.

use crate::error::WebError;
use crate::error::WebResult;
use totp_rs::Algorithm;
use totp_rs::Secret;
use totp_rs::TOTP;

/// Data returned to the user when setting up TOTP authentication.
#[derive(Debug, Clone)]
pub struct TotpSetupData {
    /// Raw base32 encoded secret key (for manual entry and storage).
    pub secret: String,
    /// Base32 secret formatted into 4-character blocks for readability.
    pub formatted_secret: String,
    /// Scannable QR code formatted as a PNG data URL (`data:image/png;base64,...`).
    pub qr_data_url: String,
}

/// Generates a new TOTP secret and scannable QR code for a user account.
///
/// # Errors
/// Returns an error if the secret generation or QR rendering fails.
pub fn generate_totp_setup(user_email: &str) -> WebResult<TotpSetupData> {
    let secret = Secret::generate_secret();
    let secret_encoded_str = secret.to_encoded().to_string();

    let secret_bytes = secret
        .to_bytes()
        .map_err(|e| WebError::Internal(format!("Failed to parse TOTP secret: {e}")))?;

    let totp = TOTP::new(
        Algorithm::SHA512,
        8,
        1,  // 1 step (30s) tolerance before and after current time
        30, // 30 second time step
        secret_bytes,
        Some("Apich Organization Security Team".to_string()),
        user_email.to_string(),
    )
    .map_err(|e| WebError::Internal(format!("Failed to initialize TOTP: {e}")))?;

    let qr_base64 = totp
        .get_qr_base64()
        .map_err(|e| WebError::Internal(format!("Failed to render TOTP QR code: {e}")))?;
    let qr_data_url = format!("data:image/png;base64,{qr_base64}");

    // Format secret into 4-char chunks (e.g. "ABCD EFGH IJKL MNOP") for manual entry
    let formatted_secret = secret_encoded_str
        .as_bytes()
        .chunks(4)
        .map(|chunk| std::str::from_utf8(chunk).unwrap_or(""))
        .collect::<Vec<_>>()
        .join(" ");

    Ok(TotpSetupData {
        secret: secret_encoded_str,
        formatted_secret,
        qr_data_url,
    })
}

/// Verifies a 8-digit TOTP code against a user's base32 secret key.
pub fn verify_totp(secret_encoded: &str, code: &str) -> bool {
    let clean_code = code.trim().replace(' ', "");
    let clean_secret = secret_encoded.trim().replace(' ', "").to_uppercase();

    let Ok(secret_bytes) = Secret::Encoded(clean_secret).to_bytes() else {
        return false;
    };

    let Ok(totp) = TOTP::new(
        Algorithm::SHA512,
        8,
        1,  // 1 step (+/- 30s) tolerance
        30,
        secret_bytes,
        Some("Apich Organization Security Team".to_string()),
        "".to_string(),
    ) else {
        return false;
    };

    totp.check_current(&clean_code).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_totp_sha512_8digits_lifecycle() {
        let setup = generate_totp_setup("researcher@apich.org").expect("setup generation failed");
        assert!(!setup.secret.is_empty());
        assert!(setup.qr_data_url.starts_with("data:image/png;base64,"));
        assert!(!setup.formatted_secret.is_empty());

        let secret_bytes = Secret::Encoded(setup.secret.clone()).to_bytes().unwrap();
        let totp = TOTP::new(
            Algorithm::SHA512,
            8,
            1,
            30,
            secret_bytes,
            Some("Apich Organization Security Team".to_string()),
            "researcher@apich.org".to_string(),
        )
        .unwrap();

        let current_code = totp.generate_current().expect("token generation failed");
        assert_eq!(current_code.len(), 8);

        // Verification must succeed
        assert!(verify_totp(&setup.secret, &current_code));

        // Invalid code must fail
        assert!(!verify_totp(&setup.secret, "00000000"));
        assert!(!verify_totp(&setup.secret, "123456"));
    }
}

