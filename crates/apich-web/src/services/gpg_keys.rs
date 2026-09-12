//! GPG public key fingerprint parser.
//!
//! Parses an ASCII-armored GPG public key's fingerprint without importing it into
//! a keyring, using `gpg --show-keys` to inspect key material directly.

use std::io::Write;
use std::process::Command;
use std::process::Stdio;

/// Parse the primary key fingerprint out of an ASCII-armored GPG public key block.
///
/// # Errors
///
/// Returns an error if spawning `gpg` fails, piping fails, or no fingerprint could be parsed.
pub fn gpg_key_fingerprint(armored: &str) -> Result<String, String> {
    let mut cmd = Command::new("gpg");
    cmd.arg("--with-colons").arg("--show-keys");
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    let Some(mut stdin) = child.stdin.take() else {
        return Err("stdin was not piped".to_string());
    };
    stdin
        .write_all(armored.as_bytes())
        .map_err(|e| e.to_string())?;
    drop(stdin);
    let output = child.wait_with_output().map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned());
    }

    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .find_map(|l| l.strip_prefix("fpr:::::::::"))
        .map(|rest| rest.trim_end_matches(':').to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "no fingerprint found in key material".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Stdio as StdStdio;

    #[test]
    fn test_gpg_key_fingerprint_parses_real_key() {
        let tmp = tempfile::tempdir().unwrap();
        let gnupghome = tmp.path();

        let batch = "\
%no-protection
Key-Type: EDDSA
Key-Curve: Ed25519
Key-Usage: sign
Name-Real: Fingerprint Test
Name-Email: fp@apich.local
Expire-Date: 0
%commit
";
        let mut gen_cmd = Command::new("gpg");
        gen_cmd
            .env("GNUPGHOME", gnupghome)
            .arg("--batch")
            .arg("--gen-key");
        gen_cmd
            .stdin(StdStdio::piped())
            .stdout(StdStdio::null())
            .stderr(StdStdio::piped());
        let mut child = gen_cmd.spawn().unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(batch.as_bytes())
            .unwrap();
        let out = child.wait_with_output().unwrap();
        assert!(
            out.status.success(),
            "gpg --gen-key failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );

        let export_out = Command::new("gpg")
            .env("GNUPGHOME", gnupghome)
            .arg("--batch")
            .arg("--armor")
            .arg("--export")
            .arg("fp@apich.local")
            .output()
            .unwrap();
        let armored = String::from_utf8_lossy(&export_out.stdout);
        assert!(armored.contains("BEGIN PGP PUBLIC KEY BLOCK"));

        let fingerprint = gpg_key_fingerprint(&armored).expect("parse fingerprint");
        assert_eq!(fingerprint.len(), 40);
        assert!(fingerprint.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_gpg_key_fingerprint_rejects_garbage() {
        let result = gpg_key_fingerprint("not a real key");
        assert!(result.is_err());
    }
}
