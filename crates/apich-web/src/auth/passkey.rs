use crate::error::{WebError, WebResult};
use base64::prelude::*;
use p256::ecdsa::{signature::Verifier, Signature, VerifyingKey};
use p256::pkcs8::DecodePublicKey;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use uuid::Uuid;

#[derive(Clone, Debug)]
struct ChallengeEntry {
    user_id: Option<Uuid>,
    created_at: Instant,
}

#[derive(Clone, Default)]
pub struct PasskeyManager {
    challenges: Arc<Mutex<HashMap<String, ChallengeEntry>>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PublicKeyCredentialCreationOptions {
    pub challenge: String,
    pub rp: RelyingPartyInfo,
    pub user: UserInfo,
    pub pub_key_cred_params: Vec<PubKeyCredParam>,
    pub timeout: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RelyingPartyInfo {
    pub name: String,
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub name: String,
    pub display_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PubKeyCredParam {
    #[serde(rename = "type")]
    pub cred_type: String,
    pub alg: i32, // -7 for ES256 (P-256 with SHA-256)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PublicKeyCredentialRequestOptions {
    pub challenge: String,
    pub timeout: u64,
    pub rp_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ClientDataJson {
    #[serde(rename = "type")]
    pub ceremony_type: String,
    pub challenge: String,
    pub origin: String,
}

impl PasskeyManager {
    pub fn new() -> Self {
        Self {
            challenges: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Generate a cryptographically secure 32-byte random challenge
    pub fn generate_challenge(&self, user_id: Option<Uuid>) -> String {
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        let challenge = BASE64_URL_SAFE_NO_PAD.encode(bytes);

        let mut lock = self.challenges.lock().unwrap();
        // Prune expired challenges (> 5 mins)
        lock.retain(|_, v| v.created_at.elapsed() < Duration::from_secs(300));
        lock.insert(
            challenge.clone(),
            ChallengeEntry {
                user_id,
                created_at: Instant::now(),
            },
        );

        challenge
    }

    /// Verify and consume a challenge
    pub fn verify_and_consume_challenge(&self, challenge: &str) -> WebResult<Option<Uuid>> {
        let mut lock = self.challenges.lock().unwrap();
        match lock.remove(challenge) {
            Some(entry) => {
                if entry.created_at.elapsed() > Duration::from_secs(300) {
                    return Err(WebError::PasskeyError("Challenge expired".to_string()));
                }
                Ok(entry.user_id)
            }
            None => Err(WebError::PasskeyError("Invalid or unknown challenge".to_string())),
        }
    }

    /// Verify WebAuthn Assertion ECDSA P-256 signature over (auth_data || sha256(client_data_json))
    pub fn verify_assertion(
        &self,
        public_key_bytes: &[u8],
        auth_data_bytes: &[u8],
        client_data_json: &[u8],
        signature_bytes: &[u8],
        expected_challenge: &str,
    ) -> WebResult<bool> {
        // 1. Parse client data JSON and verify challenge
        let client_data: ClientDataJson = serde_json::from_slice(client_data_json)
            .map_err(|e| WebError::PasskeyError(format!("Malformed clientDataJSON: {}", e)))?;

        if client_data.challenge != expected_challenge {
            return Err(WebError::PasskeyError("Challenge mismatch".to_string()));
        }

        // 2. Compute signature payload: auth_data || sha256(client_data_json)
        let client_data_hash = Sha256::digest(client_data_json);
        let mut signed_data = Vec::with_capacity(auth_data_bytes.len() + 32);
        signed_data.extend_from_slice(auth_data_bytes);
        signed_data.extend_from_slice(&client_data_hash);

        // 3. Parse P-256 VerifyingKey (supports SEC1 uncompressed/compressed or DER)
        let verifying_key = VerifyingKey::from_sec1_bytes(public_key_bytes)
            .or_else(|_| VerifyingKey::from_public_key_der(public_key_bytes))
            .map_err(|e| WebError::PasskeyError(format!("Invalid public key: {}", e)))?;

        // 4. Parse ECDSA Signature (supports ASN.1 DER or IEEE P1363 raw r||s)
        let signature = Signature::from_der(signature_bytes)
            .or_else(|_| Signature::from_slice(signature_bytes))
            .map_err(|e| WebError::PasskeyError(format!("Invalid signature format: {}", e)))?;

        // 5. Verify ECDSA signature
        match verifying_key.verify(&signed_data, &signature) {
            Ok(()) => Ok(true),
            Err(e) => Err(WebError::PasskeyError(format!("Signature verification failed: {}", e))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use p256::ecdsa::{signature::Signer, SigningKey};
    use serde_json::json;

    #[test]
    fn test_passkey_challenge_and_assertion_verification() {
        let manager = PasskeyManager::new();
        let user_id = Uuid::now_v7();
        let challenge = manager.generate_challenge(Some(user_id));
        assert!(!challenge.is_empty());

        let consumed = manager.verify_and_consume_challenge(&challenge).unwrap();
        assert_eq!(consumed, Some(user_id));

        // Generate P-256 key pair
        let signing_key = SigningKey::from_slice(&[7u8; 32]).unwrap();
        let verifying_key = signing_key.verifying_key();
        let pub_key_bytes = verifying_key.to_sec1_bytes();

        let client_data_json = json!({
            "type": "webauthn.get",
            "challenge": "test_challenge_123",
            "origin": "http://localhost:8080"
        })
        .to_string()
        .into_bytes();

        let auth_data = vec![1u8; 37]; // 37 bytes auth data

        let mut signed_data = Vec::new();
        signed_data.extend_from_slice(&auth_data);
        let client_hash = Sha256::digest(&client_data_json);
        signed_data.extend_from_slice(&client_hash);

        let signature: Signature = signing_key.sign(&signed_data);
        let sig_der = signature.to_der();

        let result = manager.verify_assertion(
            &pub_key_bytes,
            &auth_data,
            &client_data_json,
            sig_der.as_bytes(),
            "test_challenge_123",
        );
        assert!(result.is_ok());
        assert!(result.unwrap());
    }
}

