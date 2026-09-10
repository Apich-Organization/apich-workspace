//! GPG signing and verification for apich-vcs snapshots, shelling out to the real `gpg` binary
//! rather than reimplementing OpenPGP -- signing always happens against the *caller's own* real
//! keyring (so a private key never has to be handed to this library), while verification always
//! runs against an isolated, throwaway `GNUPGHOME` seeded only with the one public key being
//! checked, so a verification result reflects trust in that specific registered key alone, never
//! whatever else happens to be in the local user's ambient keyring.

use crate::error::{Result, VcsError};
use std::io::Write;
use std::process::{Command, Stdio};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignatureStatus {
    /// Signature verified against the given public key; carries the signing key's fingerprint.
    Valid { fingerprint: String },
    /// A signature is present but did not verify (wrong key, tampered payload, expired key, ...).
    Invalid(String),
}

/// Detached-sign `payload` using the local `gpg` keyring. `key_id` selects which local secret key
/// to sign with (a fingerprint, key ID, or email `gpg` can resolve) -- if `None`, `gpg` falls back
/// to its own configured default key. Returns the ASCII-armored detached signature.
pub fn sign_payload(payload: &[u8], key_id: Option<&str>) -> Result<String> {
    sign_payload_with_home(payload, key_id, None)
}

/// Same as `sign_payload`, but lets the caller pin `GNUPGHOME` explicitly instead of inheriting
/// the process's ambient environment. Production code never needs this (a real signer just wants
/// their own real keyring); it exists so tests can use an isolated per-test keyring without
/// mutating process-global environment state, which isn't safe across Rust's parallel test
/// execution within one binary (a real bug this crate's own tests hit: two `#[test]`s racing on
/// `std::env::set_var("GNUPGHOME", ..)` intermittently broke each other).
pub(crate) fn sign_payload_with_home(payload: &[u8], key_id: Option<&str>, gnupghome: Option<&std::path::Path>) -> Result<String> {
    let mut cmd = Command::new("gpg");
    cmd.arg("--batch").arg("--yes").arg("--detach-sign").arg("--armor");
    if let Some(k) = key_id {
        cmd.arg("--local-user").arg(k);
    }
    if let Some(home) = gnupghome {
        cmd.env("GNUPGHOME", home);
    }
    cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(VcsError::Io)?;
    child
        .stdin
        .take()
        .expect("stdin was piped")
        .write_all(payload)
        .map_err(VcsError::Io)?;
    let output = child.wait_with_output().map_err(VcsError::Io)?;

    if !output.status.success() {
        return Err(VcsError::Internal(format!(
            "gpg --detach-sign failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Verify `signature_armored` over `payload` against `public_key_armored`, using a fresh
/// throwaway keyring for the duration of this call only.
pub fn verify_signature(
    payload: &[u8],
    signature_armored: &str,
    public_key_armored: &str,
) -> Result<SignatureStatus> {
    let tmp = tempfile::tempdir().map_err(VcsError::Io)?;
    let gnupghome = tmp.path();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(gnupghome, std::fs::Permissions::from_mode(0o700)).map_err(VcsError::Io)?;
    }

    let mut import_cmd = Command::new("gpg");
    import_cmd
        .env("GNUPGHOME", gnupghome)
        .arg("--batch")
        .arg("--yes")
        .arg("--import");
    import_cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    let mut import_child = import_cmd.spawn().map_err(VcsError::Io)?;
    import_child
        .stdin
        .take()
        .expect("stdin was piped")
        .write_all(public_key_armored.as_bytes())
        .map_err(VcsError::Io)?;
    let import_out = import_child.wait_with_output().map_err(VcsError::Io)?;
    if !import_out.status.success() {
        return Ok(SignatureStatus::Invalid(format!(
            "failed to import public key: {}",
            String::from_utf8_lossy(&import_out.stderr)
        )));
    }

    let sig_path = gnupghome.join("sig.asc");
    let payload_path = gnupghome.join("payload.bin");
    std::fs::write(&sig_path, signature_armored).map_err(VcsError::Io)?;
    std::fs::write(&payload_path, payload).map_err(VcsError::Io)?;

    let verify_out = Command::new("gpg")
        .env("GNUPGHOME", gnupghome)
        .arg("--batch")
        .arg("--status-fd")
        .arg("1")
        .arg("--verify")
        .arg(&sig_path)
        .arg(&payload_path)
        .output()
        .map_err(VcsError::Io)?;

    let status_text = String::from_utf8_lossy(&verify_out.stdout);
    if verify_out.status.success() {
        let fingerprint = status_text
            .lines()
            .find_map(|l| l.strip_prefix("[GNUPG:] VALIDSIG "))
            .and_then(|rest| rest.split_whitespace().next())
            .unwrap_or_default()
            .to_string();
        Ok(SignatureStatus::Valid { fingerprint })
    } else {
        Ok(SignatureStatus::Invalid(String::from_utf8_lossy(&verify_out.stderr).into_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Stdio as StdStdio;

    /// Generates a throwaway test keypair in an isolated GNUPGHOME and returns
    /// (gnupghome tempdir, key id, armored public key).
    fn make_test_key() -> (tempfile::TempDir, String, String) {
        let tmp = tempfile::tempdir().unwrap();
        let gnupghome = tmp.path();

        let batch = "\
%no-protection
Key-Type: EDDSA
Key-Curve: Ed25519
Key-Usage: sign
Name-Real: APICH Test Key
Name-Email: test@apich.local
Expire-Date: 0
%commit
";
        let mut cmd = Command::new("gpg");
        cmd.env("GNUPGHOME", gnupghome)
            .arg("--batch")
            .arg("--gen-key");
        cmd.stdin(StdStdio::piped()).stdout(StdStdio::null()).stderr(StdStdio::piped());
        let mut child = cmd.spawn().expect("spawn gpg --gen-key");
        child.stdin.take().unwrap().write_all(batch.as_bytes()).unwrap();
        let out = child.wait_with_output().unwrap();
        assert!(out.status.success(), "gpg --gen-key failed: {}", String::from_utf8_lossy(&out.stderr));

        let list_out = Command::new("gpg")
            .env("GNUPGHOME", gnupghome)
            .arg("--batch")
            .arg("--with-colons")
            .arg("--list-secret-keys")
            .output()
            .unwrap();
        let list_text = String::from_utf8_lossy(&list_out.stdout);
        let key_id = list_text
            .lines()
            .find_map(|l| l.strip_prefix("fpr:::::::::"))
            .and_then(|rest| rest.split(':').next())
            .expect("parse key fingerprint")
            .to_string();

        let export_out = Command::new("gpg")
            .env("GNUPGHOME", gnupghome)
            .arg("--batch")
            .arg("--armor")
            .arg("--export")
            .arg(&key_id)
            .output()
            .unwrap();
        let public_key = String::from_utf8_lossy(&export_out.stdout).into_owned();

        (tmp, key_id, public_key)
    }

    #[test]
    fn test_sign_and_verify_roundtrip_with_real_gpg() {
        let (gnupghome_dir, key_id, public_key) = make_test_key();

        let payload = b"apich-vcs-snapshot-v1\nmessage:test snapshot\n";
        let sig = sign_payload_with_home(payload, Some(&key_id), Some(gnupghome_dir.path())).expect("sign_payload");
        assert!(sig.contains("BEGIN PGP SIGNATURE"));

        let status = verify_signature(payload, &sig, &public_key).expect("verify_signature");
        match status {
            SignatureStatus::Valid { fingerprint } => {
                assert_eq!(fingerprint.to_uppercase(), key_id.to_uppercase());
            }
            SignatureStatus::Invalid(msg) => panic!("expected valid signature, got invalid: {msg}"),
        }
    }

    #[test]
    fn test_verify_rejects_tampered_payload() {
        let (gnupghome_dir, key_id, public_key) = make_test_key();

        let payload = b"apich-vcs-snapshot-v1\nmessage:original\n";
        let sig = sign_payload_with_home(payload, Some(&key_id), Some(gnupghome_dir.path())).expect("sign_payload");

        let tampered = b"apich-vcs-snapshot-v1\nmessage:tampered\n";
        let status = verify_signature(tampered, &sig, &public_key).expect("verify_signature");
        assert!(matches!(status, SignatureStatus::Invalid(_)), "expected tampered payload to fail verification");
    }
}
