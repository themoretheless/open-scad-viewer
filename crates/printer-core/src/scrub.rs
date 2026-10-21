//! Redact secrets from error messages before they leave the crate.

/// Replace known secrets with `[redacted]` (case-sensitive exact substrings).
pub fn scrub_secrets(message: &str, secrets: &[&str]) -> String {
    if !secrets.iter().any(|s| !s.is_empty() && message.contains(s)) {
        return message.to_owned();
    }
    let mut out = String::with_capacity(message.len());
    let mut i = 0;
    while i < message.len() {
        let rest = &message[i..];
        let mut matched_len = 0;
        for &secret in secrets {
            if !secret.is_empty() && rest.starts_with(secret) && secret.len() > matched_len {
                matched_len = secret.len();
            }
        }
        if matched_len > 0 {
            out.push_str("[redacted]");
            i += matched_len;
        } else {
            let ch = rest.chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
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
