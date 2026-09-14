//! Redact secrets from error messages before they leave the crate.

/// Replace known secrets with `[redacted]` (case-sensitive exact substrings).
pub fn scrub_secrets(message: &str, secrets: &[&str]) -> String {
    let mut out = message.to_owned();
    for secret in secrets {
        if secret.is_empty() {
            continue;
        }
        if out.contains(secret) {
            out = out.replace(secret, "[redacted]");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrubs_access_code_and_api_key() {
        let msg = scrub_secrets(
            "login failed for user bblp pass=abcd1234 key=SECRETKEY99",
            &["abcd1234", "SECRETKEY99"],
        );
        assert!(!msg.contains("abcd1234"));
        assert!(!msg.contains("SECRETKEY99"));
        assert!(msg.contains("[redacted]"));
    }
}
